#!/usr/bin/env python3
"""
Mihomo Manager 配置同步工具
支持跨平台配置标准化和一致性检查

使用方法:
    python sync-config.py --check     # 检查配置状态
    python sync-config.py --sync      # 标准化配置
    python sync-config.py --reset     # 重置为模板配置
    python sync-config.py --report    # 生成配置报告
"""

import json
import yaml
import argparse
import sys
import os
from pathlib import Path
from typing import Dict, List, Any, Optional
import platform
from datetime import datetime

class ConfigSyncer:
    def __init__(self):
        self.platform = platform.system().lower()
        self.config_paths = self._get_config_paths()
        
    def _get_config_paths(self) -> Dict[str, Path]:
        """获取各平台的配置路径"""
        home = Path.home()
        
        paths = {
            'linux': home / '.config' / 'mihomo' / 'config.yaml',
            'darwin': home / '.config' / 'mihomo' / 'config.yaml',  # macOS
            'windows': Path(os.environ.get('APPDATA', '')) / 'mihomo' / 'config.yaml'
        }
        
        return {
            'config': paths.get(self.platform, paths['linux']),
            'template': Path(__file__).parent.parent / 'config.example.yaml',
            'backup_dir': paths.get(self.platform, paths['linux']).parent / 'backups'
        }
    
    def check_config_status(self) -> Dict[str, Any]:
        """检查配置状态"""
        config_file = self.config_paths['config']
        
        if not config_file.exists():
            return {
                'exists': False,
                'is_standardized': False,
                'issues': ['配置文件不存在'],
                'platform': self.platform,
                'path': str(config_file)
            }
        
        try:
            with open(config_file, 'r', encoding='utf-8') as f:
                config = yaml.safe_load(f)
        except Exception as e:
            return {
                'exists': True,
                'is_standardized': False,
                'issues': [f'配置文件解析失败: {str(e)}'],
                'platform': self.platform,
                'path': str(config_file)
            }
        
        issues = self._validate_config(config)
        
        return {
            'exists': True,
            'is_standardized': len(issues) == 0,
            'issues': issues,
            'platform': self.platform,
            'path': str(config_file),
            'version': config.get('config_version', '1.0'),
            'last_modified': datetime.fromtimestamp(config_file.stat().st_mtime).isoformat()
        }
    
    def _validate_config(self, config: Dict[str, Any]) -> List[str]:
        """验证配置完整性"""
        issues = []
        
        # 检查DNS配置
        dns = config.get('dns', {})
        if not dns.get('enable', False):
            issues.append('❌ DNS服务未启用')
        
        if dns.get('ipv6', False):
            issues.append('⚠️ IPv6已启用，可能影响解析速度')
        
        if not dns.get('prefer-h3', False):
            issues.append('❌ 未启用HTTP/3 DoH加速')
        
        if 'nameserver-policy' not in dns:
            issues.append('❌ 缺少DNS智能分流配置')
        
        if 'default-nameserver' not in dns:
            issues.append('❌ 缺少快速初始DNS配置')
        
        # 检查性能配置
        if not config.get('unified-delay', False):
            issues.append('❌ 未启用统一延迟测试')
        
        if not config.get('tcp-concurrent', False):
            issues.append('❌ 未启用TCP并发连接')
        
        # 检查路由规则
        rules = config.get('rules', [])
        if not any('geolocation-!cn' in rule for rule in rules):
            issues.append('❌ 缺少国外流量代理规则')
        
        if not any('category-ads-all' in rule for rule in rules):
            issues.append('❌ 缺少广告屏蔽规则')
        
        # 检查代理组
        proxy_groups = config.get('proxy-groups', [])
        group_names = [group.get('name', '') for group in proxy_groups]
        
        if 'PROXY' not in group_names:
            issues.append('⚠️ 缺少PROXY代理组')
        
        return issues
    
    def standardize_config(self) -> Dict[str, Any]:
        """标准化配置"""
        config_file = self.config_paths['config']
        changes = []
        
        # 创建备份
        backup_created = self._create_backup()
        
        # 加载现有配置或创建默认配置
        if config_file.exists():
            with open(config_file, 'r', encoding='utf-8') as f:
                config = yaml.safe_load(f) or {}
        else:
            config = self._load_template_config()
            changes.append('🆕 创建新的配置文件')
        
        # 标准化DNS配置
        dns_changes = self._standardize_dns(config)
        changes.extend(dns_changes)
        
        # 标准化性能配置
        perf_changes = self._standardize_performance(config)
        changes.extend(perf_changes)
        
        # 标准化路由规则
        rule_changes = self._standardize_rules(config)
        changes.extend(rule_changes)
        
        # 设置版本和平台信息
        config['config_version'] = '2.0'
        config['platform'] = self.platform
        config['last_standardized'] = datetime.utcnow().isoformat()
        
        # 保存配置
        config_file.parent.mkdir(parents=True, exist_ok=True)
        with open(config_file, 'w', encoding='utf-8') as f:
            yaml.dump(config, f, default_flow_style=False, allow_unicode=True, indent=2)
        
        # 验证标准化后的配置
        issues = self._validate_config(config)
        
        return {
            'success': True,
            'changes': changes,
            'issues': issues,
            'backup_created': backup_created,
            'config_path': str(config_file)
        }
    
    def _standardize_dns(self, config: Dict[str, Any]) -> List[str]:
        """标准化DNS配置"""
        changes = []
        
        if 'dns' not in config:
            config['dns'] = {}
        
        dns = config['dns']
        
        # 基础DNS配置
        if not dns.get('enable', False):
            dns['enable'] = True
            changes.append('✅ 启用DNS服务器')
        
        if dns.get('ipv6', True):
            dns['ipv6'] = False
            changes.append('🚀 禁用IPv6以提升解析速度')
        
        if not dns.get('prefer-h3', False):
            dns['prefer-h3'] = True
            changes.append('⚡ 启用HTTP/3 DoH加速')
        
        if dns.get('enhanced-mode') != 'fake-ip':
            dns['enhanced-mode'] = 'fake-ip'
            changes.append('🎯 设置Fake-IP增强模式')
        
        # DNS服务器配置
        if 'default-nameserver' not in dns:
            dns['default-nameserver'] = ['223.5.5.5', '119.29.29.29']
            changes.append('🏃‍♂️ 添加快速初始DNS服务器')
        
        if 'nameserver' not in dns:
            dns['nameserver'] = [
                'https://doh.pub/dns-query',
                'https://dns.alidns.com/dns-query'
            ]
            changes.append('🔐 设置主要DoH DNS服务器')
        
        if 'fallback' not in dns:
            dns['fallback'] = [
                'https://1.1.1.1/dns-query',
                'https://dns.google/dns-query'
            ]
            changes.append('🛡️ 设置防污染备用DNS')
        
        # 智能分流策略
        if 'nameserver-policy' not in dns:
            dns['nameserver-policy'] = {
                'geosite:cn,private,apple': [
                    'https://doh.pub/dns-query',
                    'https://dns.alidns.com/dns-query'
                ],
                'geosite:geolocation-!cn': [
                    'https://1.1.1.1/dns-query',
                    'https://dns.google/dns-query'
                ],
                'geosite:category-ads-all': 'rcode://success'
            }
            changes.append('🎯 配置DNS智能分流策略')
        
        return changes
    
    def _standardize_performance(self, config: Dict[str, Any]) -> List[str]:
        """标准化性能配置"""
        changes = []
        
        performance_configs = {
            'unified-delay': True,
            'tcp-concurrent': True,
            'find-process-mode': 'strict',
            'global-client-fingerprint': 'chrome',
            'keep-alive-interval': 30
        }
        
        for key, expected_value in performance_configs.items():
            if config.get(key) != expected_value:
                config[key] = expected_value
                changes.append(f'⚡ 设置{key}优化')
        
        return changes
    
    def _standardize_rules(self, config: Dict[str, Any]) -> List[str]:
        """标准化路由规则"""
        changes = []
        
        standard_rules = [
            'DOMAIN-SUFFIX,local,DIRECT',
            'DOMAIN-SUFFIX,localhost,DIRECT',
            'IP-CIDR,127.0.0.0/8,DIRECT,no-resolve',
            'IP-CIDR,172.16.0.0/12,DIRECT,no-resolve',
            'IP-CIDR,192.168.0.0/16,DIRECT,no-resolve',
            'IP-CIDR,10.0.0.0/8,DIRECT,no-resolve',
            'IP-CIDR,17.0.0.0/8,DIRECT,no-resolve',
            'IP-CIDR,100.64.0.0/10,DIRECT,no-resolve',
            'GEOIP,LAN,DIRECT,no-resolve',
            'GEOSITE,category-ads-all,ADBLOCK',
            'GEOSITE,private,DIRECT',
            'GEOSITE,cn,DIRECT',
            'GEOSITE,apple-cn,DIRECT',
            'GEOSITE,microsoft@cn,DIRECT',
            'GEOSITE,steam@cn,DIRECT',
            'GEOSITE,category-games@cn,DIRECT',
            'GEOSITE,geolocation-!cn,PROXY',
            'GEOIP,CN,DIRECT,no-resolve',
            'MATCH,PROXY'
        ]
        
        if 'rules' not in config or not config['rules']:
            config['rules'] = standard_rules
            changes.append('📋 设置标准化路由规则')
        else:
            # 检查并补充缺失的关键规则
            current_rules = config['rules']
            missing_rules = []
            
            for rule in standard_rules:
                if rule not in current_rules:
                    missing_rules.append(rule)
            
            if missing_rules:
                config['rules'].extend(missing_rules)
                changes.append(f'🔧 补充缺失路由规则: {len(missing_rules)}条')
        
        return changes
    
    def _load_template_config(self) -> Dict[str, Any]:
        """加载模板配置"""
        template_file = self.config_paths['template']
        
        if template_file.exists():
            with open(template_file, 'r', encoding='utf-8') as f:
                return yaml.safe_load(f)
        
        # 如果模板不存在，返回基础配置
        return {
            'port': 7890,
            'socks-port': 7891,
            'mixed-port': 7890,
            'allow-lan': False,
            'mode': 'rule',
            'log-level': 'info',
            'external-controller': '127.0.0.1:9090',
            'proxies': [],
            'proxy-groups': [],
            'rules': []
        }
    
    def _create_backup(self) -> bool:
        """创建配置备份"""
        config_file = self.config_paths['config']
        
        if not config_file.exists():
            return False
        
        backup_dir = self.config_paths['backup_dir']
        backup_dir.mkdir(parents=True, exist_ok=True)
        
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        backup_file = backup_dir / f'config_backup_{timestamp}.yaml'
        
        try:
            import shutil
            shutil.copy2(config_file, backup_file)
            return True
        except Exception:
            return False
    
    def reset_from_template(self) -> Dict[str, Any]:
        """从模板重置配置"""
        config_file = self.config_paths['config']
        
        # 创建备份
        backup_created = self._create_backup()
        
        # 加载模板配置
        template_config = self._load_template_config()
        
        # 设置平台信息
        template_config['platform'] = self.platform
        template_config['last_standardized'] = datetime.utcnow().isoformat()
        template_config['config_version'] = '2.0'
        
        # 保存配置
        config_file.parent.mkdir(parents=True, exist_ok=True)
        with open(config_file, 'w', encoding='utf-8') as f:
            yaml.dump(template_config, f, default_flow_style=False, allow_unicode=True, indent=2)
        
        # 验证配置
        issues = self._validate_config(template_config)
        
        return {
            'success': True,
            'changes': ['🔄 从模板重置配置'],
            'issues': issues,
            'backup_created': backup_created,
            'config_path': str(config_file)
        }
    
    def generate_report(self) -> str:
        """生成配置报告"""
        status = self.check_config_status()
        
        report = f"""
# 📊 Mihomo 配置分析报告

**生成时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
**平台**: {self.platform}
**配置文件**: {status['path']}

## 📋 基本信息
- 配置文件存在: {'✅ 是' if status['exists'] else '❌ 否'}
- 配置标准化: {'✅ 是' if status['is_standardized'] else '❌ 否'}
- 配置版本: {status.get('version', '未知')}
- 最后修改: {status.get('last_modified', '未知')}

## ⚠️ 配置问题
"""
        
        if status['issues']:
            for issue in status['issues']:
                report += f"- {issue}\n"
        else:
            report += "✅ 无配置问题\n"
        
        report += f"""
## 🎯 建议操作
"""
        
        if status['issues']:
            report += "- 🔧 运行 `python sync-config.py --sync` 标准化配置\n"
            report += "- 📖 查看配置指南了解更多优化选项\n"
        else:
            report += "✅ 配置状态良好，无需特殊操作\n"
        
        report += f"""
## 📝 配置要点
- 🧠 **DNS优化**: 三层架构确保最佳解析性能
- ⚡ **性能配置**: 启用并发连接和延迟测试优化
- 📋 **智能路由**: 基于GeoSite/GeoIP的精准分流
- 🌍 **跨平台**: 标准化配置确保不同平台一致性

---
Mihomo Manager - 让代理配置更简单
        """
        
        return report.strip()

def main():
    parser = argparse.ArgumentParser(
        description='Mihomo Manager 配置同步工具',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  python sync-config.py --check     检查配置状态
  python sync-config.py --sync      标准化当前配置  
  python sync-config.py --reset     重置为模板配置
  python sync-config.py --report    生成详细报告
        """
    )
    
    parser.add_argument('--check', action='store_true', help='检查配置状态')
    parser.add_argument('--sync', action='store_true', help='标准化配置')
    parser.add_argument('--reset', action='store_true', help='重置为模板配置')
    parser.add_argument('--report', action='store_true', help='生成配置报告')
    
    args = parser.parse_args()
    
    if not any([args.check, args.sync, args.reset, args.report]):
        parser.print_help()
        return 1
    
    syncer = ConfigSyncer()
    
    try:
        if args.check:
            status = syncer.check_config_status()
            print("📊 配置状态检查")
            print("=" * 50)
            print(f"平台: {status['platform']}")
            print(f"配置文件: {status['path']}")
            print(f"文件存在: {'✅' if status['exists'] else '❌'}")
            print(f"标准化状态: {'✅' if status['is_standardized'] else '❌'}")
            print(f"问题数量: {len(status['issues'])}")
            
            if status['issues']:
                print("\n⚠️ 发现的问题:")
                for issue in status['issues']:
                    print(f"  {issue}")
        
        elif args.sync:
            print("🔄 开始标准化配置...")
            result = syncer.standardize_config()
            
            print(f"操作结果: {'✅ 成功' if result['success'] else '❌ 失败'}")
            print(f"备份创建: {'✅' if result['backup_created'] else '❌'}")
            print(f"配置变更: {len(result['changes'])}项")
            print(f"剩余问题: {len(result['issues'])}项")
            
            if result['changes']:
                print("\n🔧 配置变更:")
                for change in result['changes']:
                    print(f"  {change}")
            
            if result['issues']:
                print("\n⚠️ 剩余问题:")
                for issue in result['issues']:
                    print(f"  {issue}")
        
        elif args.reset:
            print("🔄 从模板重置配置...")
            result = syncer.reset_from_template()
            
            print(f"操作结果: {'✅ 成功' if result['success'] else '❌ 失败'}")
            print(f"备份创建: {'✅' if result['backup_created'] else '❌'}")
            
            if result['issues']:
                print(f"\n⚠️ 注意事项: {len(result['issues'])}项")
                for issue in result['issues']:
                    print(f"  {issue}")
        
        elif args.report:
            print(syncer.generate_report())
    
    except Exception as e:
        print(f"❌ 操作失败: {str(e)}", file=sys.stderr)
        return 1
    
    return 0

if __name__ == '__main__':
    sys.exit(main())
