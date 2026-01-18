# Mihomo Manager

A modern web-based management interface for Mihomo (Clash Meta) proxy.

## Features

- 📡 **Subscription Management** - Parse and manage subscription links
- 🔄 **Node Switching** - Easy proxy node selection and switching
- 🌐 **TUN Mode Control** - Toggle TUN mode on/off
- ⚙️ **Service Mode** - Run as system service
- 🎨 **Modern UI** - Clean and responsive interface built with React and Tailwind CSS

## Prerequisites

- Node.js 16+ 
- Mihomo binary installed on system
- Administrator privileges (for TUN mode and service operations)

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd mihomo-manager
```

2. Install dependencies:
```bash
npm run install-deps
```

3. Start the application:
```bash
npm run dev
```

The application will be available at http://localhost:3000

## Configuration

The application will automatically detect your Mihomo installation or you can specify the binary path in the settings.

## Usage

1. **Add Subscriptions** - Paste subscription URLs to import proxy configurations
2. **Select Proxies** - Choose from available proxy nodes  
3. **Configure TUN** - Enable/disable TUN mode for system-wide proxy
4. **Service Mode** - Run Mihomo as a background service

## License

MIT
