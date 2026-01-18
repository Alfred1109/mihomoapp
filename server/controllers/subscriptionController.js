const express = require('express');
const axios = require('axios');
const fs = require('fs');
const path = require('path');
const yaml = require('yaml');

const router = express.Router();

// Storage for subscriptions
const SUBSCRIPTIONS_FILE = path.join(__dirname, '..', 'data', 'subscriptions.json');

// Ensure data directory exists
const dataDir = path.dirname(SUBSCRIPTIONS_FILE);
if (!fs.existsSync(dataDir)) {
  fs.mkdirSync(dataDir, { recursive: true });
}

// Load subscriptions from file
const loadSubscriptions = () => {
  try {
    if (fs.existsSync(SUBSCRIPTIONS_FILE)) {
      const data = fs.readFileSync(SUBSCRIPTIONS_FILE, 'utf8');
      return JSON.parse(data);
    }
  } catch (error) {
    console.error('Error loading subscriptions:', error);
  }
  return [];
};

// Save subscriptions to file
const saveSubscriptions = (subscriptions) => {
  try {
    fs.writeFileSync(SUBSCRIPTIONS_FILE, JSON.stringify(subscriptions, null, 2));
  } catch (error) {
    console.error('Error saving subscriptions:', error);
    throw error;
  }
};

// Parse subscription URL and extract proxy information
const parseSubscription = async (url, userAgent = 'clash') => {
  try {
    const response = await axios.get(url, {
      headers: {
        'User-Agent': userAgent
      },
      timeout: 30000
    });

    let config;
    const contentType = response.headers['content-type'] || '';
    
    if (contentType.includes('application/json')) {
      config = response.data;
    } else {
      // Try to parse as YAML
      try {
        config = yaml.parse(response.data);
      } catch (yamlError) {
        // If YAML parsing fails, try as base64 decoded content
        try {
          const decoded = Buffer.from(response.data, 'base64').toString('utf8');
          config = yaml.parse(decoded);
        } catch (base64Error) {
          throw new Error('Unable to parse subscription content as JSON, YAML, or base64 encoded YAML');
        }
      }
    }

    // Extract proxy information
    const proxies = config.proxies || [];
    const proxyGroups = config['proxy-groups'] || [];
    
    return {
      proxies,
      proxyGroups,
      rules: config.rules || [],
      rawConfig: config
    };

  } catch (error) {
    throw new Error(`Failed to fetch or parse subscription: ${error.message}`);
  }
};

// Get all subscriptions
router.get('/', (req, res) => {
  try {
    const subscriptions = loadSubscriptions();
    res.json(subscriptions);
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Add new subscription
router.post('/', async (req, res) => {
  try {
    const { name, url, userAgent } = req.body;

    if (!name || !url) {
      return res.status(400).json({ error: 'Name and URL are required' });
    }

    // Parse the subscription to validate it
    const parsedData = await parseSubscription(url, userAgent);
    
    const subscriptions = loadSubscriptions();
    const newSubscription = {
      id: Date.now().toString(),
      name,
      url,
      userAgent: userAgent || 'clash',
      createdAt: new Date().toISOString(),
      lastUpdated: new Date().toISOString(),
      proxyCount: parsedData.proxies.length,
      status: 'active'
    };

    subscriptions.push(newSubscription);
    saveSubscriptions(subscriptions);

    res.json({
      message: 'Subscription added successfully',
      subscription: newSubscription,
      parsedData
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to add subscription',
      details: error.message 
    });
  }
});

// Update subscription
router.put('/:id', async (req, res) => {
  try {
    const { id } = req.params;
    const { name, url, userAgent } = req.body;

    const subscriptions = loadSubscriptions();
    const subscriptionIndex = subscriptions.findIndex(sub => sub.id === id);

    if (subscriptionIndex === -1) {
      return res.status(404).json({ error: 'Subscription not found' });
    }

    // Parse the subscription to validate it if URL changed
    let parsedData = null;
    if (url && url !== subscriptions[subscriptionIndex].url) {
      parsedData = await parseSubscription(url, userAgent);
    }

    // Update subscription
    const updatedSubscription = {
      ...subscriptions[subscriptionIndex],
      ...(name && { name }),
      ...(url && { url }),
      ...(userAgent && { userAgent }),
      lastUpdated: new Date().toISOString(),
      ...(parsedData && { proxyCount: parsedData.proxies.length })
    };

    subscriptions[subscriptionIndex] = updatedSubscription;
    saveSubscriptions(subscriptions);

    res.json({
      message: 'Subscription updated successfully',
      subscription: updatedSubscription,
      ...(parsedData && { parsedData })
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to update subscription',
      details: error.message 
    });
  }
});

// Delete subscription
router.delete('/:id', (req, res) => {
  try {
    const { id } = req.params;
    const subscriptions = loadSubscriptions();
    const filteredSubscriptions = subscriptions.filter(sub => sub.id !== id);

    if (filteredSubscriptions.length === subscriptions.length) {
      return res.status(404).json({ error: 'Subscription not found' });
    }

    saveSubscriptions(filteredSubscriptions);
    res.json({ message: 'Subscription deleted successfully' });

  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// Refresh subscription (re-fetch and parse)
router.post('/:id/refresh', async (req, res) => {
  try {
    const { id } = req.params;
    const subscriptions = loadSubscriptions();
    const subscription = subscriptions.find(sub => sub.id === id);

    if (!subscription) {
      return res.status(404).json({ error: 'Subscription not found' });
    }

    // Re-fetch and parse the subscription
    const parsedData = await parseSubscription(subscription.url, subscription.userAgent);
    
    // Update subscription info
    subscription.lastUpdated = new Date().toISOString();
    subscription.proxyCount = parsedData.proxies.length;
    subscription.status = 'active';

    saveSubscriptions(subscriptions);

    res.json({
      message: 'Subscription refreshed successfully',
      subscription,
      parsedData
    });

  } catch (error) {
    // Update subscription status to error
    const subscriptions = loadSubscriptions();
    const subscription = subscriptions.find(sub => sub.id === id);
    if (subscription) {
      subscription.status = 'error';
      subscription.lastError = error.message;
      subscription.lastUpdated = new Date().toISOString();
      saveSubscriptions(subscriptions);
    }

    res.status(500).json({ 
      error: 'Failed to refresh subscription',
      details: error.message 
    });
  }
});

// Get subscription content
router.get('/:id/content', async (req, res) => {
  try {
    const { id } = req.params;
    const subscriptions = loadSubscriptions();
    const subscription = subscriptions.find(sub => sub.id === id);

    if (!subscription) {
      return res.status(404).json({ error: 'Subscription not found' });
    }

    const parsedData = await parseSubscription(subscription.url, subscription.userAgent);
    res.json(parsedData);

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to fetch subscription content',
      details: error.message 
    });
  }
});

// Parse subscription URL without saving
router.post('/parse', async (req, res) => {
  try {
    const { url, userAgent } = req.body;

    if (!url) {
      return res.status(400).json({ error: 'URL is required' });
    }

    const parsedData = await parseSubscription(url, userAgent);
    res.json(parsedData);

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to parse subscription',
      details: error.message 
    });
  }
});

// Generate merged configuration from subscriptions
router.post('/merge', async (req, res) => {
  try {
    const { subscriptionIds, mergeOptions } = req.body;

    if (!subscriptionIds || !Array.isArray(subscriptionIds) || subscriptionIds.length === 0) {
      return res.status(400).json({ error: 'Subscription IDs are required' });
    }

    const subscriptions = loadSubscriptions();
    const selectedSubscriptions = subscriptions.filter(sub => 
      subscriptionIds.includes(sub.id) && sub.status === 'active'
    );

    if (selectedSubscriptions.length === 0) {
      return res.status(400).json({ error: 'No valid subscriptions found' });
    }

    // Fetch and merge all subscription content
    const allProxies = [];
    const allProxyGroups = [];
    const allRules = [];

    for (const subscription of selectedSubscriptions) {
      try {
        const parsedData = await parseSubscription(subscription.url, subscription.userAgent);
        allProxies.push(...parsedData.proxies);
        allProxyGroups.push(...parsedData.proxyGroups);
        allRules.push(...parsedData.rules);
      } catch (error) {
        console.error(`Failed to fetch subscription ${subscription.name}:`, error);
      }
    }

    // Create merged configuration
    const mergedConfig = {
      proxies: allProxies,
      'proxy-groups': allProxyGroups,
      rules: allRules,
      ...mergeOptions
    };

    res.json({
      message: 'Subscriptions merged successfully',
      config: mergedConfig,
      stats: {
        totalProxies: allProxies.length,
        totalGroups: allProxyGroups.length,
        totalRules: allRules.length
      }
    });

  } catch (error) {
    res.status(500).json({ 
      error: 'Failed to merge subscriptions',
      details: error.message 
    });
  }
});

module.exports = router;
