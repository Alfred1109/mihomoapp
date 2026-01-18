const express = require('express');
const { spawn, exec } = require('child_process');
const fs = require('fs');
const path = require('path');
const axios = require('axios');
const yaml = require('yaml');

const router = express.Router();

// Mihomo binary paths (will be auto-detected or configurable)
const POSSIBLE_MIHOMO_PATHS = [
  'mihomo',
  'mihomo.exe', 
  'clash-meta',
  'clash-meta.exe',
  '/usr/local/bin/mihomo',
  '/usr/bin/mihomo',
  'C:\\Program Files\\mihomo\\mihomo.exe'
];

// Configuration paths
const CONFIG_PATHS = [
  path.join(process.env.HOME || process.env.USERPROFILE || '.', '.config', 'mihomo'),
  path.join(process.env.HOME || process.env.USERPROFILE || '.', '.mihomo'),
  path.join(__dirname, '..', 'config')
];

let mihomoProcess = null;
let mihomoPath = null;
let configPath = null;

// Initialize mihomo path and config directory
const initializePaths = () => {
  // Find mihomo binary
  for (const testPath of POSSIBLE_MIHOMO_PATHS) {
    try {
      exec(`${testPath} -v`, (error) => {
        if (!error && !mihomoPath) {
          mihomoPath = testPath;
          console.log(`Found mihomo at: ${mihomoPath}`);
        }
      });
    } catch (err) {
      // Continue checking other paths
    }
  }

  // Find or create config directory
  for (const testPath of CONFIG_PATHS) {
    if (fs.existsSync(testPath)) {
      configPath = testPath;
      break;
    }
  }

  if (!configPath) {
    configPath = CONFIG_PATHS[0];
    fs.mkdirSync(configPath, { recursive: true });
  }

  console.log(`Config directory: ${configPath}`);
};

initializePaths();

// Get mihomo status
router.get('/status', async (req, res) => {
  try {
    const isRunning = mihomoProcess && !mihomoProcess.killed;
    let apiStatus = null;
    
    if (isRunning) {
      try {
        // Try to connect to mihomo API (default port 9090)
        const response = await axios.get('http://127.0.0.1:9090/configs', {
          timeout: 2000
        });
        apiStatus = response.data;
      } catch (error) {
        console.log('Could not connect to mihomo API:', error.message);
      }
    }

    res.json({
      isRunning,
      processId: mihomoProcess?.pid || null,
      apiStatus,
      mihomoPath,
      configPath
    });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Start mihomo service
router.post('/start', async (req, res) => {
  try {
    if (mihomoProcess && !mihomoProcess.killed) {
      return res.status(400).json({ error: 'Mihomo is already running' });
    }

    if (!mihomoPath) {
      return res.status(400).json({ error: 'Mihomo binary not found. Please install mihomo first.' });
    }

    const configFile = path.join(configPath, 'config.yaml');
    if (!fs.existsSync(configFile)) {
      return res.status(400).json({ error: 'Configuration file not found. Please setup configuration first.' });
    }

    // Start mihomo process
    mihomoProcess = spawn(mihomoPath, ['-d', configPath], {
      stdio: ['pipe', 'pipe', 'pipe'],
      detached: false
    });

    let startupOutput = '';
    
    mihomoProcess.stdout.on('data', (data) => {
      const output = data.toString();
      startupOutput += output;
      console.log('Mihomo stdout:', output);
      
      // Broadcast to WebSocket clients
      if (req.app.locals.wss) {
        req.app.locals.wss.clients.forEach(client => {
          if (client.readyState === 1) {
            client.send(JSON.stringify({
              type: 'mihomo_log',
              data: output
            }));
          }
        });
      }
    });

    mihomoProcess.stderr.on('data', (data) => {
      const output = data.toString();
      console.error('Mihomo stderr:', output);
      
      // Broadcast to WebSocket clients
      if (req.app.locals.wss) {
        req.app.locals.wss.clients.forEach(client => {
          if (client.readyState === 1) {
            client.send(JSON.stringify({
              type: 'mihomo_error',
              data: output
            }));
          }
        });
      }
    });

    mihomoProcess.on('close', (code) => {
      console.log(`Mihomo process exited with code ${code}`);
      mihomoProcess = null;
      
      // Broadcast to WebSocket clients
      if (req.app.locals.wss) {
        req.app.locals.wss.clients.forEach(client => {
          if (client.readyState === 1) {
            client.send(JSON.stringify({
              type: 'mihomo_stopped',
              data: { code }
            }));
          }
        });
      }
    });

    // Wait a bit to see if process starts successfully
    await new Promise(resolve => setTimeout(resolve, 2000));

    if (mihomoProcess && !mihomoProcess.killed) {
      res.json({ 
        message: 'Mihomo started successfully',
        processId: mihomoProcess.pid,
        startupOutput
      });
    } else {
      res.status(500).json({ 
        error: 'Failed to start mihomo',
        startupOutput
      });
    }

  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Stop mihomo service
router.post('/stop', (req, res) => {
  try {
    if (!mihomoProcess || mihomoProcess.killed) {
      return res.status(400).json({ error: 'Mihomo is not running' });
    }

    mihomoProcess.kill('SIGTERM');
    
    // Force kill after 5 seconds if not stopped
    setTimeout(() => {
      if (mihomoProcess && !mihomoProcess.killed) {
        mihomoProcess.kill('SIGKILL');
      }
    }, 5000);

    res.json({ message: 'Mihomo stopped successfully' });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Restart mihomo service
router.post('/restart', async (req, res) => {
  try {
    // Stop first
    if (mihomoProcess && !mihomoProcess.killed) {
      mihomoProcess.kill('SIGTERM');
      
      // Wait for process to stop
      await new Promise(resolve => {
        if (mihomoProcess) {
          mihomoProcess.on('close', resolve);
        } else {
          resolve();
        }
      });
    }

    // Wait a bit before restarting
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Start again
    req.url = '/start';
    req.method = 'POST';
    return router.handle(req, res);
    
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Get proxies from mihomo API
router.get('/proxies', async (req, res) => {
  try {
    const response = await axios.get('http://127.0.0.1:9090/proxies', {
      timeout: 5000
    });
    res.json(response.data);
  } catch (error) {
    res.status(500).json({ 
      error: 'Could not fetch proxies from mihomo API',
      details: error.message 
    });
  }
});

// Switch proxy for a group
router.put('/proxies/:group', async (req, res) => {
  try {
    const { group } = req.params;
    const { name } = req.body;

    if (!name) {
      return res.status(400).json({ error: 'Proxy name is required' });
    }

    const response = await axios.put(`http://127.0.0.1:9090/proxies/${group}`, 
      { name },
      { timeout: 5000 }
    );
    
    res.json({ message: `Switched ${group} to ${name}` });
  } catch (error) {
    res.status(500).json({ 
      error: 'Could not switch proxy',
      details: error.message 
    });
  }
});

// Get current configuration
router.get('/config', async (req, res) => {
  try {
    const response = await axios.get('http://127.0.0.1:9090/configs', {
      timeout: 5000
    });
    res.json(response.data);
  } catch (error) {
    res.status(500).json({ 
      error: 'Could not fetch configuration from mihomo API',
      details: error.message 
    });
  }
});

// Update configuration
router.patch('/config', async (req, res) => {
  try {
    const config = req.body;
    
    const response = await axios.patch('http://127.0.0.1:9090/configs', config, {
      timeout: 5000,
      headers: {
        'Content-Type': 'application/json'
      }
    });
    
    res.json({ message: 'Configuration updated successfully' });
  } catch (error) {
    res.status(500).json({ 
      error: 'Could not update configuration',
      details: error.message 
    });
  }
});

// Toggle TUN mode
router.post('/tun/:action', async (req, res) => {
  try {
    const { action } = req.params;
    
    if (!['enable', 'disable'].includes(action)) {
      return res.status(400).json({ error: 'Action must be enable or disable' });
    }

    const tunEnabled = action === 'enable';
    
    const response = await axios.patch('http://127.0.0.1:9090/configs', {
      tun: {
        enable: tunEnabled
      }
    }, {
      timeout: 5000,
      headers: {
        'Content-Type': 'application/json'
      }
    });
    
    res.json({ 
      message: `TUN mode ${action}d successfully`,
      tunEnabled 
    });
  } catch (error) {
    res.status(500).json({ 
      error: `Could not ${req.params.action} TUN mode`,
      details: error.message 
    });
  }
});

// Get traffic statistics
router.get('/traffic', async (req, res) => {
  try {
    const response = await axios.get('http://127.0.0.1:9090/traffic', {
      timeout: 5000
    });
    res.json(response.data);
  } catch (error) {
    res.status(500).json({ 
      error: 'Could not fetch traffic statistics',
      details: error.message 
    });
  }
});

// Get logs
router.get('/logs', async (req, res) => {
  try {
    const response = await axios.get('http://127.0.0.1:9090/logs', {
      timeout: 5000
    });
    res.json(response.data);
  } catch (error) {
    res.status(500).json({ 
      error: 'Could not fetch logs',
      details: error.message 
    });
  }
});

module.exports = router;
