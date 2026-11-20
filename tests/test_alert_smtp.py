"""SMTP transport contract; no network or real credentials."""
import contextlib
import io
from pathlib import Path
import smtplib
import ssl
import sys
import unittest
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts/production'))
import notify_failure as alerts


class MailTests(unittest.TestCase):
    def setUp(self):
        self.config = dict(smtp_host='smtp.example.invalid', smtp_port=587,
                           smtp_mode='starttls', smtp_username='fixture',
                           smtp_password='PRIVATE_FIXTURE', to=['ops@example.invalid'])
        self.config['from'] = 'monitor@example.invalid'
        self.body = dict(event_id='synthetic-event', event='monitor.delivery.test')
        self.context = ssl.create_default_context()

    def test_tls_before_authentication(self):
        client = MagicMock()
        client.send_message.return_value = {}
        with patch.object(alerts.smtplib, 'SMTP') as constructor:
            constructor.return_value.__enter__.return_value = client
            self.assertTrue(alerts.send_mail(self.config, self.context, self.body))
        self.assertEqual([c[0] for c in client.method_calls],
                         ['ehlo', 'starttls', 'ehlo', 'login', 'send_message'])
        self.assertIs(client.starttls.call_args.kwargs['context'], self.context)

    def test_failed_tls_never_authenticates(self):
        client = MagicMock()
        client.starttls.side_effect = ssl.SSLCertVerificationError('PRIVATE_FIXTURE')
        output = io.StringIO()
        with patch.object(alerts.smtplib, 'SMTP') as constructor, contextlib.redirect_stderr(output):
            constructor.return_value.__enter__.return_value = client
            self.assertFalse(alerts.send_mail(self.config, self.context, self.body))
        client.login.assert_not_called()
        client.send_message.assert_not_called()
        self.assertNotIn('PRIVATE_FIXTURE', output.getvalue())

    def test_auth_rejection_does_not_retry_or_log_server_response(self):
        client = MagicMock()
        client.login.side_effect = smtplib.SMTPAuthenticationError(535, b'PRIVATE_FIXTURE')
        output = io.StringIO()
        with patch.object(alerts.smtplib, 'SMTP') as constructor, contextlib.redirect_stderr(output):
            constructor.return_value.__enter__.return_value = client
            self.assertFalse(alerts.send_mail(self.config, self.context, self.body))
            self.assertEqual(constructor.call_count, 1)
        client.send_message.assert_not_called()
        self.assertNotIn('PRIVATE_FIXTURE', output.getvalue())


if __name__ == '__main__':
    unittest.main()
