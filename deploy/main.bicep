// Provisions the whole Release 1 stack on Azure Container Apps:
//   - a Container Apps environment (with its Log Analytics workspace)
//   - the backend app (internal ingress, scales to zero)
//   - the nginx web app (external ingress, scales to zero, proxies /api to the
//     backend over its internal FQDN)
//
// Postgres is external (Neon), passed in as a secure connection string.

@description('Azure region for all resources.')
param location string = resourceGroup().location

@description('GitHub owner whose GHCR hosts the images, e.g. mahamadahir.')
param ghcrOwner string

@description('Image tag to deploy (a release tag such as v1.0.0, or latest).')
param imageTag string = 'latest'

@description('Neon direct (non-pooled) connection string, with sslmode=require.')
@secure()
param databaseUrl string

@description('Prefix for resource names.')
param namePrefix string = 'permission-ledger'

@description('Address to email when an alert fires. Leave empty to skip alerting.')
param alertEmail string = ''

// Container image references must be lowercase, but GitHub usernames are
// case-preserving, so normalise rather than relying on how it was typed.
var owner = toLower(ghcrOwner)
var backendImage = 'ghcr.io/${owner}/permission-ledger-backend:${imageTag}'
var webImage = 'ghcr.io/${owner}/permission-ledger-web:${imageTag}'

resource logs 'Microsoft.OperationalInsights/workspaces@2023-09-01' = {
  name: '${namePrefix}-logs'
  location: location
  properties: {
    sku: { name: 'PerGB2018' }
    retentionInDays: 30
  }
}

resource env 'Microsoft.App/managedEnvironments@2024-03-01' = {
  name: '${namePrefix}-env'
  location: location
  properties: {
    appLogsConfiguration: {
      destination: 'log-analytics'
      logAnalyticsConfiguration: {
        customerId: logs.properties.customerId
        sharedKey: logs.listKeys().primarySharedKey
      }
    }
  }
}

resource backend 'Microsoft.App/containerApps@2024-03-01' = {
  name: '${namePrefix}-backend'
  location: location
  properties: {
    managedEnvironmentId: env.id
    configuration: {
      // Internal ingress: reachable only from the web app inside the
      // environment, never from the public internet.
      ingress: {
        external: false
        targetPort: 3000
        transport: 'http'
      }
      secrets: [
        { name: 'database-url', value: databaseUrl }
      ]
    }
    template: {
      containers: [
        {
          name: 'backend'
          image: backendImage
          resources: { cpu: json('0.25'), memory: '0.5Gi' }
          env: [
            { name: 'DATABASE_URL', secretRef: 'database-url' }
            { name: 'BACKEND_BIND_ADDR', value: '0.0.0.0:3000' }
            { name: 'COOKIE_SECURE', value: 'true' }
            // Same-origin, so CORS is never exercised; this just needs to parse.
            { name: 'WEB_ORIGIN', value: 'https://${namePrefix}-web.${env.properties.defaultDomain}' }
            { name: 'RUST_LOG', value: 'permission_ledger_backend=info,tower_http=warn' }
          ]
          probes: [
            {
              type: 'Liveness'
              httpGet: { path: '/health', port: 3000 }
              initialDelaySeconds: 5
              periodSeconds: 30
            }
          ]
        }
      ]
      scale: { minReplicas: 0, maxReplicas: 1 }
    }
  }
}

resource web 'Microsoft.App/containerApps@2024-03-01' = {
  name: '${namePrefix}-web'
  location: location
  properties: {
    managedEnvironmentId: env.id
    configuration: {
      ingress: {
        external: true
        targetPort: 80
        transport: 'http'
      }
    }
    template: {
      containers: [
        {
          name: 'web'
          image: webImage
          resources: { cpu: json('0.25'), memory: '0.5Gi' }
          env: [
            // The backend's internal ingress serves HTTPS on its FQDN.
            { name: 'BACKEND_UPSTREAM', value: 'https://${backend.properties.configuration.ingress.fqdn}' }
            { name: 'NGINX_ENVSUBST_FILTER', value: 'BACKEND_UPSTREAM' }
          ]
        }
      ]
      scale: { minReplicas: 0, maxReplicas: 3 }
    }
  }
}

// Alerting is only wired up when an address is supplied, so the stack can be
// deployed without it.
var alertsEnabled = !empty(alertEmail)

resource alertGroup 'Microsoft.Insights/actionGroups@2023-01-01' = if (alertsEnabled) {
  name: '${namePrefix}-alerts'
  location: 'global'
  properties: {
    groupShortName: 'plalerts'
    enabled: true
    emailReceivers: [
      {
        name: 'owner'
        emailAddress: alertEmail
        useCommonAlertSchema: true
      }
    ]
  }
}

// Server errors: the app is up but failing requests.
resource serverErrorAlert 'Microsoft.Insights/metricAlerts@2018-03-01' = if (alertsEnabled) {
  name: '${namePrefix}-backend-5xx'
  location: 'global'
  properties: {
    description: 'The backend returned server errors in the last 5 minutes.'
    severity: 1
    enabled: true
    scopes: [backend.id]
    evaluationFrequency: 'PT5M'
    windowSize: 'PT5M'
    criteria: {
      'odata.type': 'Microsoft.Azure.Monitor.SingleResourceMultipleMetricCriteria'
      allOf: [
        {
          name: 'ServerErrors'
          metricNamespace: 'Microsoft.App/containerApps'
          metricName: 'Requests'
          operator: 'GreaterThan'
          threshold: 0
          timeAggregation: 'Total'
          criterionType: 'StaticThresholdCriterion'
          dimensions: [
            {
              name: 'statusCodeCategory'
              operator: 'Include'
              values: ['5xx']
            }
          ]
        }
      ]
    }
    actions: [{ actionGroupId: alertGroup.id }]
  }
}

// Restarts: a crash loop, most likely a bad migration or an unreachable database.
resource restartAlert 'Microsoft.Insights/metricAlerts@2018-03-01' = if (alertsEnabled) {
  name: '${namePrefix}-backend-restarts'
  location: 'global'
  properties: {
    description: 'The backend container restarted, which usually means it cannot start cleanly.'
    severity: 1
    enabled: true
    scopes: [backend.id]
    evaluationFrequency: 'PT5M'
    windowSize: 'PT15M'
    criteria: {
      'odata.type': 'Microsoft.Azure.Monitor.SingleResourceMultipleMetricCriteria'
      allOf: [
        {
          name: 'Restarts'
          metricNamespace: 'Microsoft.App/containerApps'
          metricName: 'RestartCount'
          operator: 'GreaterThan'
          threshold: 2
          timeAggregation: 'Maximum'
          criterionType: 'StaticThresholdCriterion'
        }
      ]
    }
    actions: [{ actionGroupId: alertGroup.id }]
  }
}

@description('Public HTTPS URL of the dashboard.')
output webUrl string = 'https://${web.properties.configuration.ingress.fqdn}'

@description('Internal FQDN the web app proxies to.')
output backendFqdn string = backend.properties.configuration.ingress.fqdn
