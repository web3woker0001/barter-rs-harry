# Cloudflare Tunnel 快速部署指南

## 🚀 一键部署

```bash
# 自动部署（替换 example.com 为您的域名）
./cloudflare/deploy.sh --domain example.com
```

## 📋 前置要求

1. **Cloudflare 账号**（免费即可）
2. **域名** 已添加到 Cloudflare
3. **Docker** 和 **Docker Compose**

## 🔧 手动部署步骤

### 1. 安装 cloudflared

```bash
# macOS
brew install cloudflare/cloudflare/cloudflared

# Ubuntu/Debian
wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb

# CentOS/RHEL
wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-x86_64.rpm
sudo rpm -i cloudflared-linux-x86_64.rpm
```

### 2. 登录 Cloudflare

```bash
cloudflared tunnel login
# 浏览器会打开，选择您的域名授权
```

### 3. 创建 Tunnel

```bash
# 创建 tunnel
cloudflared tunnel create crypto-monitor

# 查看 tunnel ID
cloudflared tunnel list
# 记录 ID，例如: 6ff42ae2-765d-4adf-8112-31c55c1551ef
```

### 4. 配置 DNS

```bash
# 批量添加 DNS 记录
cloudflared tunnel route dns crypto-monitor monitor.example.com
cloudflared tunnel route dns crypto-monitor api.monitor.example.com
cloudflared tunnel route dns crypto-monitor ws.monitor.example.com
cloudflared tunnel route dns crypto-monitor grafana.monitor.example.com
```

### 5. 更新配置文件

编辑 `cloudflare/config.yml`：
- 替换 `YOUR_TUNNEL_ID` 为实际的 Tunnel ID
- 替换 `example.com` 为您的域名

### 6. 复制凭证

```bash
# 复制 tunnel 凭证到项目目录
cp ~/.cloudflared/YOUR_TUNNEL_ID.json cloudflare/credentials/
```

### 7. 启动服务

```bash
# 启动所有服务（包括 Cloudflare Tunnel）
docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml up -d

# 或使用 Makefile
make cloudflare-up
```

## 🌐 访问地址

部署成功后，可通过以下地址访问：

| 服务 | URL | 说明 |
|------|-----|------|
| 主站 | https://monitor.example.com | Web 界面 |
| API | https://api.monitor.example.com | REST API |
| WebSocket | wss://ws.monitor.example.com | 实时数据 |
| Grafana | https://grafana.monitor.example.com | 监控面板 |
| 健康检查 | https://health.monitor.example.com | 服务状态 |

## 🔒 安全配置

### 启用 Cloudflare Access（可选）

1. 登录 [Cloudflare Zero Trust](https://one.dash.cloudflare.com/)
2. Access → Applications → Add Application
3. 选择 Self-hosted
4. 配置访问策略：

```yaml
Application name: Crypto Monitor Admin
Application domain: admin.monitor.example.com
Policy name: Admin Access
Rule: 
  - Emails ending in: @yourcompany.com
  - Require MFA: Yes
```

### 配置 WAF 规则

在 Cloudflare Dashboard：

1. **Security → WAF → Custom Rules**
```
IF URI Path contains "/api/trading/"
THEN Challenge
```

2. **Security → Rate Limiting**
```
Path: /api/*
Rate: 100 requests per minute
Action: Block for 1 minute
```

## 📊 监控

### 查看 Tunnel 状态

```bash
# 查看日志
docker-compose logs -f cloudflared

# 查看指标
curl http://localhost:2000/metrics

# 查看连接状态
cloudflared tunnel info crypto-monitor
```

### Grafana Dashboard

访问 https://grafana.monitor.example.com

默认登录：
- 用户名: admin
- 密码: admin

## 🔧 常用命令

```bash
# 查看 tunnel 列表
cloudflared tunnel list

# 删除 tunnel
cloudflared tunnel delete crypto-monitor

# 更新 DNS
cloudflared tunnel route dns crypto-monitor monitor.example.com --overwrite-dns

# 验证配置
cloudflared tunnel --config cloudflare/config.yml ingress validate

# 测试运行（不后台）
cloudflared tunnel --config cloudflare/config.yml run
```

## 🐛 故障排查

### 1. Tunnel 无法连接

```bash
# 检查凭证
ls -la cloudflare/credentials/

# 测试连接
cloudflared tunnel --config cloudflare/config.yml run --loglevel debug

# 重新认证
cloudflared tunnel login
```

### 2. DNS 无法解析

```bash
# 检查 DNS 记录
dig monitor.example.com
nslookup monitor.example.com

# 检查 Cloudflare DNS
curl -X GET "https://api.cloudflare.com/client/v4/zones/ZONE_ID/dns_records" \
  -H "X-Auth-Email: your-email@example.com" \
  -H "X-Auth-Key: your-api-key"
```

### 3. 503 错误

检查后端服务：
```bash
# 检查服务状态
docker-compose ps

# 测试后端
curl http://localhost:8080/health

# 查看容器日志
docker-compose logs crypto-monitor
```

## 🔄 更新部署

```bash
# 拉取最新代码
git pull

# 重建镜像
docker-compose build

# 重启服务
docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml down
docker-compose -f docker-compose.yml -f docker-compose.cloudflare.yml up -d
```

## 📈 性能优化

### Cloudflare 设置

1. **Speed → Optimization**
   - Auto Minify: ON (JavaScript, CSS, HTML)
   - Brotli: ON
   - Rocket Loader: ON

2. **Caching → Configuration**
   - Browser Cache TTL: 4 hours
   - Always Online: ON

3. **Network**
   - HTTP/3 (with QUIC): ON
   - 0-RTT Connection Resumption: ON
   - WebSockets: ON

### 负载均衡（企业版）

```yaml
# cloudflare/lb-config.yml
pools:
  - name: crypto-monitor-pool
    origins:
      - address: crypto-monitor-1:8080
        weight: 1
      - address: crypto-monitor-2:8080
        weight: 1
    monitor:
      type: http
      path: /health
      interval: 60
```

## 📝 环境变量

创建 `.env.cloudflare`：

```bash
# Cloudflare 配置
CF_TUNNEL_NAME=crypto-monitor
CF_TUNNEL_ID=your-tunnel-id
CF_DOMAIN=example.com

# 可选：API 配置
CF_API_EMAIL=your-email@example.com
CF_API_KEY=your-api-key
CF_ZONE_ID=your-zone-id

# 可选：Access 配置
CF_ACCESS_CLIENT_ID=your-client-id
CF_ACCESS_CLIENT_SECRET=your-client-secret
```

## 🎯 最佳实践

1. **使用 Cloudflare Access** 保护管理端点
2. **启用 Rate Limiting** 防止 API 滥用
3. **配置 Page Rules** 优化缓存
4. **使用 Transform Rules** 添加安全头
5. **监控 Analytics** 了解流量模式
6. **定期更新** cloudflared 版本

## 📚 相关资源

- [Cloudflare Tunnel 文档](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/)
- [Zero Trust 文档](https://developers.cloudflare.com/cloudflare-one/)
- [WAF 配置指南](https://developers.cloudflare.com/waf/)
- [性能优化指南](https://developers.cloudflare.com/fundamentals/get-started/speed/)