const express = require('express');
const fs = require('fs');
const path = require('path');
const yaml = require('yaml');

const router = express.Router();

// Configuration paths
const getConfigPath = () => {
  const possiblePaths = [
    path.join(process.env.HOME || process.env.USERPROFILE || '.', '.config', 'mihomo'),
    path.join(process.env.HOME || process.env.USERPROFILE || '.', '.mihomo'),
    path.join(__dirname, '..', 'config')
  ];

  for (const testPath of possiblePaths) {
    if (fs.existsSync(testPath)) {
      return testPath;
    }
  }

  // Create default config directory
  const defaultPath = possiblePaths[0];
  fs.mkdirSync(defaultPath, { recursive: true });
  return defaultPath;
};

// Default mihomo configuration template
const getDefaultConfig = () => ({
  port: 7890,
  'socks-port': 7891,
  'redir-port': 7892,
  'tproxy-port': 7893,
  'mixed-port': 7890,
  'allow-lan': false,
  'bind-address': '*',
  mode: 'rule',
  'log-level': 'info',
  ipv6: true,
  'external-controller': '127.0.0.1:9090',
  'external-ui': 'ui',
  secret: '',
  'interface-name': '',
  'routing-mark': 6666,
  tun: {
    enable: false,
    stack: 'system',
    'dns-hijack': ['8.8.8.8:53', '8.8.4.4:53'],
    'auto-route': true,
    'auto-detect-interface': true
  },
  dns: {
    enable: true,
    listen: '0.0.0.0:53',
    'default-nameserver': ['223.5.5.5', '119.29.29.29'],
    enhanced_mode: 'fake-ip',
    'fake-ip-range': '198.18.0.1/16',
    'use-hosts': true,
    nameserver: ['https://doh.pub/dns-query', 'https://dns.alidns.com/dns-query'],
    fallback: ['https://1.1.1.1/dns-query', 'https://dns.google/dns-query'],
    'fallback-filter': {
      geoip: true,
      'geoip-code': 'CN',
      ipcidr: ['240.0.0.0/4']
    }
  },
  proxies: [],
  'proxy-groups': [
    {
      name: 'PROXY',
      type: 'select',
      proxies: ['DIRECT']
    }
  ],
  rules: [
    'MATCH,PROXY'
  ]
});

// Get current configuration
router.get('/', (req, res) => {
  try {
    const configPath = getConfigPath();
    const configFile = path.join(configPath, 'config.yaml');

    if (fs.existsSync(configFile)) {
      const configContent = fs.readFileSync(configFile, 'utf8');
      const config = yaml.parse(configContent);
      res.json({
        config,
        configPath: configFile,
        exists: true
      });
    } else {
      const defaultConfig = getDefaultConfig();
      res.json({
        config: defaultConfig,
        configPath: configFile,
        exists: false
      });
    }
  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to read configuration',
      details: error.message 
    });
  }
});

// Save configuration
router.post('/', (req, res) => {
  try {
    const { config } = req.body;

    if (!config) {
      return res.status(400).json({ error: 'Configuration is required' });
    }

    const configPath = getConfigPath();
    const configFile = path.join(configPath, 'config.yaml');

    // Validate configuration structure
    const requiredFields = ['port', 'mode', 'proxies', 'proxy-groups', 'rules'];
    for (const field of requiredFields) {
      if (!(field in config)) {
        return res.status(400).json({ 
          error: `Missing required field: ${field}` 
        });
      }
    }

    // Convert to YAML and save
    const yamlContent = yaml.stringify(config, {
      indent: 2,
      lineWidth: 0,
      minContentWidth: 0
    });

    fs.writeFileSync(configFile, yamlContent, 'utf8');

    res.json({
      message: 'Configuration saved successfully',
      configPath: configFile
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to save configuration',
      details: error.message 
    });
  }
});

// Update specific configuration fields
router.patch('/', (req, res) => {
  try {
    const updates = req.body;
    const configPath = getConfigPath();
    const configFile = path.join(configPath, 'config.yaml');

    let currentConfig;

    if (fs.existsSync(configFile)) {
      const configContent = fs.readFileSync(configFile, 'utf8');
      currentConfig = yaml.parse(configContent);
    } else {
      currentConfig = getDefaultConfig();
    }

    // Apply updates
    const updatedConfig = { ...currentConfig, ...updates };

    // Convert to YAML and save
    const yamlContent = yaml.stringify(updatedConfig, {
      indent: 2,
      lineWidth: 0,
      minContentWidth: 0
    });

    fs.writeFileSync(configFile, yamlContent, 'utf8');

    res.json({
      message: 'Configuration updated successfully',
      config: updatedConfig,
      configPath: configFile
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to update configuration',
      details: error.message 
    });
  }
});

// Reset configuration to default
router.post('/reset', (req, res) => {
  try {
    const configPath = getConfigPath();
    const configFile = path.join(configPath, 'config.yaml');
    const defaultConfig = getDefaultConfig();

    const yamlContent = yaml.stringify(defaultConfig, {
      indent: 2,
      lineWidth: 0,
      minContentWidth: 0
    });

    fs.writeFileSync(configFile, yamlContent, 'utf8');

    res.json({
      message: 'Configuration reset to default',
      config: defaultConfig,
      configPath: configFile
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to reset configuration',
      details: error.message 
    });
  }
});

// Backup configuration
router.post('/backup', (req, res) => {
  try {
    const configPath = getConfigPath();
    const configFile = path.join(configPath, 'config.yaml');

    if (!fs.existsSync(configFile)) {
      return res.status(404).json({ error: 'Configuration file not found' });
    }

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const backupFile = path.join(configPath, `config-backup-${timestamp}.yaml`);

    fs.copyFileSync(configFile, backupFile);

    res.json({
      message: 'Configuration backed up successfully',
      backupPath: backupFile
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to backup configuration',
      details: error.message 
    });
  }
});

// List configuration backups
router.get('/backups', (req, res) => {
  try {
    const configPath = getConfigPath();
    
    if (!fs.existsSync(configPath)) {
      return res.json({ backups: [] });
    }

    const files = fs.readdirSync(configPath);
    const backups = files
      .filter(file => file.startsWith('config-backup-') && file.endsWith('.yaml'))
      .map(file => {
        const filePath = path.join(configPath, file);
        const stats = fs.statSync(filePath);
        return {
          filename: file,
          path: filePath,
          created: stats.birthtime.toISOString(),
          size: stats.size
        };
      })
      .sort((a, b) => new Date(b.created) - new Date(a.created));

    res.json({ backups });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to list backups',
      details: error.message 
    });
  }
});

// Restore from backup
router.post('/restore/:filename', (req, res) => {
  try {
    const { filename } = req.params;
    const configPath = getConfigPath();
    const backupFile = path.join(configPath, filename);
    const configFile = path.join(configPath, 'config.yaml');

    if (!fs.existsSync(backupFile)) {
      return res.status(404).json({ error: 'Backup file not found' });
    }

    // Create a backup of current config first
    if (fs.existsSync(configFile)) {
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const currentBackup = path.join(configPath, `config-backup-before-restore-${timestamp}.yaml`);
      fs.copyFileSync(configFile, currentBackup);
    }

    // Restore from backup
    fs.copyFileSync(backupFile, configFile);

    // Read and return the restored configuration
    const configContent = fs.readFileSync(configFile, 'utf8');
    const config = yaml.parse(configContent);

    res.json({
      message: 'Configuration restored successfully',
      config,
      configPath: configFile
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to restore configuration',
      details: error.message 
    });
  }
});

// Validate configuration
router.post('/validate', (req, res) => {
  try {
    const { config } = req.body;

    if (!config) {
      return res.status(400).json({ error: 'Configuration is required' });
    }

    const errors = [];
    const warnings = [];

    // Required fields validation
    const requiredFields = ['port', 'mode', 'proxies', 'proxy-groups', 'rules'];
    for (const field of requiredFields) {
      if (!(field in config)) {
        errors.push(`Missing required field: ${field}`);
      }
    }

    // Port validation
    if (config.port && (config.port < 1 || config.port > 65535)) {
      errors.push('Port must be between 1 and 65535');
    }

    // Mode validation
    if (config.mode && !['rule', 'global', 'direct'].includes(config.mode)) {
      errors.push('Mode must be one of: rule, global, direct');
    }

    // Proxy groups validation
    if (config['proxy-groups'] && Array.isArray(config['proxy-groups'])) {
      config['proxy-groups'].forEach((group, index) => {
        if (!group.name) {
          errors.push(`Proxy group ${index + 1} missing name`);
        }
        if (!group.type) {
          errors.push(`Proxy group "${group.name || index + 1}" missing type`);
        }
        if (!group.proxies || !Array.isArray(group.proxies)) {
          errors.push(`Proxy group "${group.name || index + 1}" missing or invalid proxies array`);
        }
      });
    }

    // TUN mode validation
    if (config.tun && config.tun.enable && process.platform !== 'win32') {
      warnings.push('TUN mode may require administrator privileges on non-Windows systems');
    }

    const isValid = errors.length === 0;

    res.json({
      valid: isValid,
      errors,
      warnings
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to validate configuration',
      details: error.message 
    });
  }
});

module.exports = router;
