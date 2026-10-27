import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('sync_config', Path(__file__).with_name('sync-config.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class FingerprintCompatibilityTests(unittest.TestCase):
    def test_legacy_fallback_keeps_explicit_none_and_shadowtls_only(self):
        config = {'global-client-fingerprint': 'chrome', 'proxies': [
            {'type': 'vless'}, {'type': 'trojan', 'client-fingerprint': 'none'},
            {'type': 'ss', 'plugin': 'shadow-tls'}, {'type': 'ss', 'plugin': 'restls'},
            {'type': 'vless', 'client-fingerprint': 'firefox', 'xhttp-opts': {'download-settings': {'client-fingerprint': ''}}},
        ]}
        self.assertTrue(module.migrate_client_fingerprint(config))
        self.assertNotIn('global-client-fingerprint', config)
        self.assertEqual(config['proxies'][0]['client-fingerprint'], 'chrome')
        self.assertEqual(config['proxies'][1]['client-fingerprint'], 'none')
        self.assertEqual(config['proxies'][2]['client-fingerprint'], 'chrome')
        self.assertNotIn('client-fingerprint', config['proxies'][3])
        self.assertEqual(config['proxies'][4]['xhttp-opts']['download-settings']['client-fingerprint'], 'chrome')
        self.assertFalse(module.migrate_client_fingerprint(config))

    def test_provider_expressions_are_appended_once(self):
        config = {'global-client-fingerprint': 'firefox', 'proxy-providers': {'p': {'override': {'udp': True, 'override-expr': ['.name += " suffix"']}}}}
        module.migrate_client_fingerprint(config)
        expressions = config['proxy-providers']['p']['override']['override-expr']
        self.assertEqual(len(expressions), 3)
        self.assertEqual(expressions[0], '.name += " suffix"')
        self.assertIn('gost-relay', expressions[1])
        self.assertFalse(module.migrate_client_fingerprint(config))

    def test_conflict_keeps_input_unchanged(self):
        config = {'global-client-fingerprint': 'chrome', module.DEFAULT_FINGERPRINT_KEY: 'none'}
        original = config.copy()
        with self.assertRaises(ValueError):
            module.migrate_client_fingerprint(config)
        self.assertEqual(config, original)


if __name__ == '__main__':
    unittest.main()
