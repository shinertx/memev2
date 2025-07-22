# 🚀 GO LIVE CHECKLIST - MemeSnipe v18

## 📋 Pre-Deployment Checklist

### ✅ **System Setup**
- [ ] Fresh GCP instance or clean environment
- [ ] Git repository cloned
- [ ] Deployment script permissions set (`chmod +x deploy.sh`)

### ✅ **Dependencies**
- [ ] Docker installed
- [ ] Rust installed  
- [ ] System dependencies installed
- [ ] Python 3 installed

## � Deployment Process

### **1. Initial Deployment**
```bash
# Clone and deploy
git clone https://github.com/shinertx/memev2.git
cd memev2/meme-snipe-v18
chmod +x deploy.sh
./deploy.sh deploy
```

### **2. Verify Installation**
```bash
# Check system status
./deploy.sh status

# Check health
./deploy.sh health

# View logs
./deploy.sh logs
```

### **3. Access Points**
- **Dashboard**: http://localhost
- **Health Check**: http://localhost/health
- **API**: http://localhost/api/

## 💰 Trading Configuration

### **1. Wallet Setup**
```bash
# Wallet is auto-generated during deployment
# Check wallet address in logs
./deploy.sh logs signer | grep "pubkey"
```

### **2. Fund Wallet**
- [ ] Send minimum 0.1 SOL to wallet for gas fees
- [ ] Verify balance before trading
- [ ] Test with small amounts first

### **3. API Keys Configuration**
Edit `.env` file with your keys:
```bash
# Required for production
HELIUS_API_KEY=your_helius_key_here
OPENAI_API_KEY=your_openai_key_here
GROK_API_KEY=your_grok_key_here

# Optional for notifications
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
TELEGRAM_CHAT_ID=your_telegram_chat_id
DISCORD_WEBHOOK_URL=your_discord_webhook_url
```

## 🎯 Strategy Testing

### **1. Paper Trading Phase**
```bash
# Verify paper trading is enabled
grep "PAPER_TRADING_MODE=true" .env

# Monitor for 24-48 hours
./deploy.sh logs strategy_factory
./deploy.sh status
```

### **2. Performance Validation**
- [ ] Check strategy win rates in dashboard
- [ ] Verify risk management is working
- [ ] Monitor for any errors or crashes
- [ ] Validate all data sources are connected

### **3. Risk Management Check**
- [ ] Position limits working correctly
- [ ] Stop losses triggering properly
- [ ] Emergency stop functionality tested
- [ ] Daily loss limits respected

## � GO LIVE SEQUENCE

### **1. Final Pre-Live Checks**
```bash
# System health
./deploy.sh health

# All services running
./deploy.sh status

# No critical errors in logs
./deploy.sh logs | grep -i error
```

### **2. Enable Live Trading**
Edit `.env` file:
```bash
# Change these settings
PAPER_TRADING_MODE=false
ENABLE_LIVE_PORTFOLIO=true

# Start with conservative settings
MAX_POSITION_SIZE_PERCENT=5.0
MAX_DAILY_TRADES=10
MAX_DAILY_LOSS_USD=100.00
```

### **3. Restart with Live Settings**
```bash
./deploy.sh restart
./deploy.sh status
```

### **4. Initial Live Monitoring**
- [ ] Monitor first trades closely
- [ ] Check P&L in real-time
- [ ] Verify notifications are working
- [ ] Watch for any unusual behavior

## 📊 Ongoing Operations

### **Daily Checks**
```bash
# Morning routine
./deploy.sh status
./deploy.sh health
curl http://localhost/health

# Check overnight performance
./deploy.sh logs strategy_factory | tail -100
```

### **Monitoring**
- [ ] Set up alerts for system failures
- [ ] Monitor wallet balance daily
- [ ] Check strategy performance weekly
- [ ] Review and adjust risk parameters

### **Maintenance**
```bash
# Weekly updates
./deploy.sh update

# Monthly backups
./deploy.sh backup

# Quarterly cleanup
./deploy.sh clean
```

## 🚨 Emergency Procedures

### **Emergency Stop**
```bash
# Immediate stop
./deploy.sh stop

# Or set emergency stop in .env
ENABLE_EMERGENCY_STOP=true
PAPER_TRADING_MODE=true
```

### **Troubleshooting**
```bash
# Check service health
./deploy.sh health

# View detailed logs
./deploy.sh logs [service_name]

# Restart problematic services
./deploy.sh restart

# Full system restart
./deploy.sh stop
./deploy.sh start
```

## ⚠️ Important Reminders

### **Risk Management**
- Start with small position sizes
- Never risk more than you can afford to lose
- Keep emergency stop procedures ready
- Monitor performance closely

### **Security**
- Keep API keys secure
- Regular backups of wallet and data
- Monitor for unauthorized access
- Update system regularly

### **Performance**
- Paper trade for adequate time before going live
- Gradually increase position sizes
- Document and learn from all trades
- Adjust strategies based on performance

---

**🎯 Remember**: This is experimental trading software. Use at your own risk and start conservatively!

#### Option A: Docker (Recommended)
```bash
# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
# Logout/login, then:
cd /home/benjaminjones/memev2/meme-snipe-v18
make build && make up
```

#### Option B: GCP Deployment
```bash
# Install gcloud CLI, then:
./scripts/deploy_vm_gcp.sh
```

#### Option C: Manual Server Setup
Transfer files and run components individually

## 🔍 Pre-Flight Verification

### Step 1: Verify Wallet Files
```bash
# Check files exist
ls -la my_wallet.json jito_auth_key.json

# Verify wallet format (should show public key)
solana-keygen pubkey my_wallet.json
```

### Step 2: Test API Keys
```bash
# Test Helius
curl "https://rpc.helius.xyz/?api-key=YOUR_KEY" -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getSlot"}'

# Test Pyth (if applicable)
# Test Twitter API (if applicable)
```

### Step 3: Validate Configuration
```bash
# Check .env syntax
grep -v '^#' .env | grep -v '^$'

# Verify no placeholder values remain
grep "YOUR_.*_HERE" .env
```

## 🚨 FINAL SAFETY CHECKS

### Before Going Live:
1. **Start with PAPER_TRADING_MODE=true first**
2. **Test all strategies with small amounts**
3. **Monitor for 24-48 hours before scaling**
4. **Have emergency stop procedures ready**

### Emergency Controls:
```bash
# Stop all trading immediately
docker-compose down

# Check current positions
docker-compose logs executor | grep "POSITION"

# View recent trades
docker-compose logs executor | tail -100
```

## 📊 Monitoring Setup

### Dashboard Access:
- Web UI: http://localhost:8080
- Logs: `docker-compose logs -f`
- Metrics: Prometheus on port 9184

### Key Metrics to Watch:
- Total P&L
- Open positions
- Error rates
- API response times
- Wallet balance

## 🔧 Troubleshooting

### Common Issues:
1. **"Wallet file not found"** → Check file paths and permissions
2. **"API key invalid"** → Verify keys in .env file
3. **"RPC connection failed"** → Check network/RPC endpoint
4. **"Insufficient balance"** → Add SOL to wallet

### Support:
- Logs: `docker-compose logs [service-name]`
- Debug mode: Set `LOG_LEVEL=debug` in .env
- Container status: `docker-compose ps`

---

## ⚠️ DISCLAIMER
- This bot trades real money and can lose funds
- Past performance does not guarantee future results  
- Only risk what you can afford to lose
- Monitor the system actively, especially initially
- Have emergency stop procedures ready

**By proceeding, you acknowledge the risks involved in automated trading.**
