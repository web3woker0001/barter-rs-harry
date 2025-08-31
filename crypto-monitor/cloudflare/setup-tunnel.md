# Cloudflare Tunnel 部署指南

通过 Cloudflare Tunnel 安全地将 Crypto Monitor 服务暴露到互联网，无需公网 IP 和开放端口。

## 目录

1. [快速开始](#快速开始)
2. [详细配置](#详细配置)
3. [高级设置](#高级设置)
4. [安全建议](#安全建议)
5. [故障排查](#故障排查)

## 快速开始

### 前置要求

- Cloudflare 账号
- 已添加到 Cloudflare 的域名（如 example.com）
- Docker 和 Docker Compose

### 1. 安装 Cloudflare Tunnel

```bash
# macOS
brew install cloudflare/cloudflare/cloudflared

# Linux
wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb

# Docker
docker pull cloudflare/cloudflared:latest
```

### 2. 登录 Cloudflare

```bash
cloudflared tunnel login
# 浏览器会打开，选择您的域名授权
```

### 3. 创建 Tunnel

```bash
# 创建名为 crypto-monitor 的 tunnel
cloudflared tunnel create crypto-monitor

# 记录 Tunnel ID (类似: 6ff42ae2-765d-4adf-8112-31c55c1551ef)
cloudflared tunnel list
```

### 4. 配置 DNS

```bash
# 自动添加 DNS 记录
cloudflared tunnel route dns crypto-monitor monitor.example.com
cloudflared tunnel route dns crypto-monitor api.monitor.example.com
cloudflared tunnel route dns crypto-monitor ws.monitor.example.com
```

或在 Cloudflare Dashboard 手动添加：
- Type: CNAME
- Name: monitor
- Target: `<TUNNEL_ID>.cfargotunnel.com`

## 详细配置

### 配置文件结构

创建 `cloudflare/config.yml`：

```yaml
# Cloudflare Tunnel 配置
tunnel: 6ff42ae2-765d-4adf-8112-31c55c1551ef  # 替换为您的 Tunnel ID
credentials-file: /home/user/.cloudflared/6ff42ae2-765d-4adf-8112-31c55c1551ef.json

# 入口规则
ingress:
  # API 服务
  - hostname: api.monitor.example.com
    service: http://localhost:8080
    originRequest:
      noTLSVerify: false
      connectTimeout: 30s
      
  # WebSocket 服务
  - hostname: ws.monitor.example.com
    service: ws://localhost:8081
    originRequest:
      noTLSVerify: false
      
  # Grafana 监控
  - hostname: grafana.monitor.example.com
    service: http://localhost:3000
    originRequest:
      noTLSVerify: false
      
  # 主域名 - Web UI
  - hostname: monitor.example.com
    service: http://localhost:80
    originRequest:
      noTLSVerify: false
      
  # 404 规则（必须）
  - service: http_status:404
```

### Docker Compose 集成

创建 `docker-compose.cloudflare.yml`：

```yaml
version: '3.9'

services:
  cloudflared:
    image: cloudflare/cloudflared:latest
    container_name: crypto-monitor-tunnel
    restart: unless-stopped
    command: tunnel run
    volumes:
      - ./cloudflare/config.yml:/etc/cloudflared/config.yml:ro
      - ./cloudflare/credentials:/etc/cloudflared/credentials:ro
    networks:
      - crypto-monitor-network
    depends_on:
      - crypto-monitor
      - nginx

networks:
  crypto-monitor-network:
    external: true
```

### 启动服务

```bash
# 使用 Docker Compose
docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml up -d

# 或手动运行
cloudflared tunnel --config cloudflare/config.yml run
```

## 高级设置

### 1. 访问控制 (Zero Trust)

创建 `cloudflare/access-policy.yml`：

```yaml
# Cloudflare Access 策略
policies:
  # API 访问策略
  - name: API Access
    hostname: api.monitor.example.com
    include:
      - email:
          - "*@yourcompany.com"
      - ip:
          - "192.168.1.0/24"
    require:
      - purpose: "api_access"
      
  # 管理面板访问
  - name: Admin Access
    hostname: monitor.example.com
    path: /admin/*
    include:
      - email:
          - "admin@yourcompany.com"
    require:
      - mfa: true
```

应用访问策略：

```bash
# 在 Cloudflare Dashboard 中配置
# Teams → Access → Applications → Add Application
```

### 2. 负载均衡配置

```yaml
# cloudflare/config-lb.yml
tunnel: 6ff42ae2-765d-4adf-8112-31c55c1551ef
credentials-file: /etc/cloudflared/credentials.json

# 负载均衡池
originRequest:
  proxyType: http
  
ingress:
  - hostname: api.monitor.example.com
    service: lb://crypto-api-pool
    originRequest:
      noTLSVerify: false
      
  - service: http_status:404

# 定义负载均衡池
load-balancer:
  crypto-api-pool:
    - http://crypto-monitor-1:8080
    - http://crypto-monitor-2:8080
    - http://crypto-monitor-3:8080
```

### 3. WAF 规则

创建 `cloudflare/waf-rules.json`：

```json
{
  "rules": [
    {
      "expression": "(http.request.uri.path contains \"/api/\" and not ip.src in {192.168.0.0/16})",
      "action": "challenge",
      "description": "Challenge non-local API access"
    },
    {
      "expression": "(http.request.method eq \"POST\" and http.request.uri.path contains \"/trading/\")",
      "action": "managed_challenge",
      "description": "Extra security for trading endpoints"
    },
    {
      "expression": "rate.limit(100, 60)",
      "action": "block",
      "description": "Rate limit: 100 requests per minute"
    }
  ]
}
```

### 4. 监控和日志

```yaml
# cloudflare/config-monitoring.yml
tunnel: 6ff42ae2-765d-4adf-8112-31c55c1551ef
credentials-file: /etc/cloudflared/credentials.json

# 日志配置
loglevel: info
logfile: /var/log/cloudflared.log

# 指标配置
metrics: localhost:2000

ingress:
  - hostname: api.monitor.example.com
    service: http://localhost:8080
    originRequest:
      # 启用追踪
      enableTracing: true
      # 自定义头部
      httpHostHeader: "api.monitor.internal"
      originServerName: "api.monitor.example.com"
      
  - service: http_status:404
```

## 自动化部署脚本

创建 `cloudflare/deploy.sh`：

```bash
#!/bin/bash

set -e

# 配置变量
DOMAIN="example.com"
TUNNEL_NAME="crypto-monitor"
CONFIG_FILE="./cloudflare/config.yml"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}=== Cloudflare Tunnel 部署脚本 ===${NC}"

# 检查 cloudflared 是否安装
if ! command -v cloudflared &> /dev/null; then
    echo -e "${RED}cloudflared 未安装，正在安装...${NC}"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew install cloudflare/cloudflare/cloudflared
    else
        wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
        sudo dpkg -i cloudflared-linux-amd64.deb
    fi
fi

# 登录检查
echo -e "${YELLOW}检查 Cloudflare 登录状态...${NC}"
if ! cloudflared tunnel list &> /dev/null; then
    echo "请登录 Cloudflare..."
    cloudflared tunnel login
fi

# 创建或获取 tunnel
echo -e "${YELLOW}设置 Tunnel...${NC}"
if cloudflared tunnel list | grep -q "$TUNNEL_NAME"; then
    echo "Tunnel '$TUNNEL_NAME' 已存在"
    TUNNEL_ID=$(cloudflared tunnel list | grep "$TUNNEL_NAME" | awk '{print $1}')
else
    echo "创建新 Tunnel '$TUNNEL_NAME'..."
    cloudflared tunnel create "$TUNNEL_NAME"
    TUNNEL_ID=$(cloudflared tunnel list | grep "$TUNNEL_NAME" | awk '{print $1}')
fi

echo -e "${GREEN}Tunnel ID: $TUNNEL_ID${NC}"

# 创建配置文件
echo -e "${YELLOW}生成配置文件...${NC}"
cat > "$CONFIG_FILE" << EOF
tunnel: $TUNNEL_ID
credentials-file: $HOME/.cloudflared/$TUNNEL_ID.json

ingress:
  # API 服务
  - hostname: api.monitor.$DOMAIN
    service: http://localhost:8080
    originRequest:
      noTLSVerify: false
      connectTimeout: 30s
      
  # WebSocket 服务
  - hostname: ws.monitor.$DOMAIN
    service: ws://localhost:8081
    originRequest:
      noTLSVerify: false
      
  # Grafana 监控
  - hostname: grafana.monitor.$DOMAIN
    service: http://localhost:3000
    originRequest:
      noTLSVerify: false
      
  # 主域名
  - hostname: monitor.$DOMAIN
    service: http://localhost:80
    originRequest:
      noTLSVerify: false
      
  # 404 规则
  - service: http_status:404
EOF

# 配置 DNS
echo -e "${YELLOW}配置 DNS 记录...${NC}"
cloudflared tunnel route dns "$TUNNEL_NAME" "monitor.$DOMAIN" || true
cloudflared tunnel route dns "$TUNNEL_NAME" "api.monitor.$DOMAIN" || true
cloudflared tunnel route dns "$TUNNEL_NAME" "ws.monitor.$DOMAIN" || true
cloudflared tunnel route dns "$TUNNEL_NAME" "grafana.monitor.$DOMAIN" || true

# 创建 systemd 服务（Linux）
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo -e "${YELLOW}创建 systemd 服务...${NC}"
    sudo cloudflared service install
    sudo systemctl enable cloudflared
    sudo systemctl start cloudflared
fi

# 验证
echo -e "${YELLOW}验证配置...${NC}"
cloudflared tunnel --config "$CONFIG_FILE" ingress validate

echo -e "${GREEN}=== 部署完成 ===${NC}"
echo ""
echo "访问地址："
echo "  主站: https://monitor.$DOMAIN"
echo "  API: https://api.monitor.$DOMAIN"
echo "  WebSocket: wss://ws.monitor.$DOMAIN"
echo "  Grafana: https://grafana.monitor.$DOMAIN"
echo ""
echo "启动 Tunnel："
echo "  cloudflared tunnel --config $CONFIG_FILE run"
echo "或使用 Docker："
echo "  docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml up -d"
```

## Nginx 配置更新

更新 `nginx/nginx.conf` 以支持 Cloudflare：

```nginx
server {
    listen 80;
    server_name monitor.example.com api.monitor.example.com ws.monitor.example.com;

    # Cloudflare IP 验证
    set_real_ip_from 173.245.48.0/20;
    set_real_ip_from 103.21.244.0/22;
    set_real_ip_from 103.22.200.0/22;
    set_real_ip_from 103.31.4.0/22;
    set_real_ip_from 141.101.64.0/18;
    set_real_ip_from 108.162.192.0/18;
    set_real_ip_from 190.93.240.0/20;
    set_real_ip_from 188.114.96.0/20;
    set_real_ip_from 197.234.240.0/22;
    set_real_ip_from 198.41.128.0/17;
    set_real_ip_from 162.158.0.0/15;
    set_real_ip_from 104.16.0.0/13;
    set_real_ip_from 104.24.0.0/14;
    set_real_ip_from 172.64.0.0/13;
    set_real_ip_from 131.0.72.0/22;
    set_real_ip_from 2400:cb00::/32;
    set_real_ip_from 2606:4700::/32;
    set_real_ip_from 2803:f800::/32;
    set_real_ip_from 2405:b500::/32;
    set_real_ip_from 2405:8100::/32;
    set_real_ip_from 2a06:98c0::/29;
    set_real_ip_from 2c0f:f248::/32;
    
    real_ip_header CF-Connecting-IP;

    # API 路由
    location /api/ {
        proxy_pass http://crypto-monitor:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header CF-Ray $http_cf_ray;
        proxy_set_header CF-Visitor $http_cf_visitor;
    }

    # WebSocket 路由
    location /ws {
        proxy_pass http://crypto-monitor:8081;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## 安全建议

### 1. API 密钥管理

```yaml
# cloudflare/auth-config.yml
auth:
  api_keys:
    - name: "production"
      key: "${API_KEY}"
      allowed_ips:
        - "192.168.1.0/24"
      rate_limit: 1000
      
  jwt:
    secret: "${JWT_SECRET}"
    expiry: 3600
```

### 2. DDoS 防护

在 Cloudflare Dashboard 配置：

1. **Security → DDoS**
   - Sensitivity: High
   - Action: Challenge

2. **Security → WAF → Rate Limiting**
   - 10 requests/second per IP
   - 1000 requests/minute per API key

3. **Security → Bots**
   - Bot Fight Mode: On
   - Challenge Solve Rate: Aggressive

### 3. 监控告警

```yaml
# cloudflare/alerting.yml
alerts:
  - name: "High Error Rate"
    condition: "error_rate > 5%"
    notification:
      - email: "ops@example.com"
      - webhook: "https://hooks.slack.com/..."
      
  - name: "Tunnel Down"
    condition: "tunnel_status != healthy"
    notification:
      - sms: "+1234567890"
      - pagerduty: "service_key"
```

## 故障排查

### 常见问题

#### 1. Tunnel 无法连接

```bash
# 检查 tunnel 状态
cloudflared tunnel info crypto-monitor

# 查看日志
cloudflared tunnel --config cloudflare/config.yml run --loglevel debug

# 测试连接
curl -I https://monitor.example.com
```

#### 2. DNS 解析问题

```bash
# 检查 DNS 记录
dig monitor.example.com
nslookup monitor.example.com

# 刷新 DNS
cloudflared tunnel route dns crypto-monitor monitor.example.com --overwrite-dns
```

#### 3. 证书问题

```bash
# 重新生成证书
cloudflared tunnel cleanup crypto-monitor
cloudflared tunnel create crypto-monitor

# 验证证书
openssl s_client -connect monitor.example.com:443
```

### 日志位置

- Cloudflared: `/var/log/cloudflared.log`
- Docker: `docker logs crypto-monitor-tunnel`
- Systemd: `journalctl -u cloudflared -f`

## 监控集成

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Cloudflare Tunnel Metrics",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [{
          "expr": "rate(cloudflared_tunnel_requests_total[5m])"
        }]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "rate(cloudflared_tunnel_errors_total[5m])"
        }]
      },
      {
        "title": "Latency",
        "targets": [{
          "expr": "histogram_quantile(0.95, cloudflared_tunnel_latency_seconds)"
        }]
      }
    ]
  }
}
```

## 完整部署命令

```bash
# 1. 克隆仓库
git clone https://github.com/your-repo/crypto-monitor.git
cd crypto-monitor

# 2. 配置环境
cp .env.example .env
# 编辑 .env 文件

# 3. 部署 Cloudflare Tunnel
chmod +x cloudflare/deploy.sh
./cloudflare/deploy.sh

# 4. 启动所有服务
docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml up -d

# 5. 验证
curl https://monitor.example.com/health
```

## 访问地址

部署完成后，可通过以下地址访问：

- 🌐 主站: https://monitor.example.com
- 🔌 API: https://api.monitor.example.com
- 📡 WebSocket: wss://ws.monitor.example.com
- 📊 Grafana: https://grafana.monitor.example.com

所有流量都通过 Cloudflare 的全球网络，提供 DDoS 防护、WAF、CDN 加速等功能。