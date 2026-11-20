#!/usr/bin/env python3
"""Send a minimal operational event using HTTPS or authenticated TLS mail."""
import argparse
import datetime
import json
import os
from pathlib import Path
import re
import socket
import smtplib
from email.message import EmailMessage
from email.utils import parseaddr
import ssl
import stat
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from backup import regular, trusted_parents


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def configuration(path):
    if not path.is_absolute():
        raise ValueError('Configuration path must be absolute')
    trusted_parents(path)
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, 'rb') as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o077 or info.st_size > 65536:
            raise ValueError('Configuration must be root-owned, private and at most 64 KiB')
        config = json.loads(source.read(65537))
    if not isinstance(config, dict): raise ValueError('Configuration must be an object')
    transport = config.get('transport','webhook')
    fields = {'transport','ca_file'} | ({'url','bearer_token'} if transport == 'webhook' else
              {'smtp_host','smtp_port','smtp_mode','smtp_username','smtp_password','from','to'})
    if transport not in ('webhook','smtp') or set(config)-fields:
        raise ValueError('Unknown configuration fields')
    if transport == 'smtp':
        host = config.get('smtp_host')
        if not isinstance(host,str) or not host or any(c.isspace() or c in '/@' for c in host): raise ValueError('Invalid SMTP host')
        if config.get('smtp_mode') not in ('starttls','tls'): raise ValueError('SMTP TLS is mandatory')
        if type(config.get('smtp_port')) is not int or not 1 <= config['smtp_port'] <= 65535: raise ValueError('Invalid SMTP port')
        recipients = config.get('to')
        if not isinstance(recipients,list) or not 1 <= len(recipients) <= 10: raise ValueError('Specify 1..10 recipients')
        for address in [config.get('from'),*recipients]:
            if not isinstance(address,str) or not address.isascii() or '@' not in address or any(c.isspace() for c in address) or parseaddr(address)[1]!=address:
                raise ValueError('Use bare email addresses without display names')
        if bool(config.get('smtp_username')) != bool(config.get('smtp_password')): raise ValueError('Provide username and password together')
        for field in ('smtp_username','smtp_password'):
            if field in config and (not isinstance(config[field],str) or '\r' in config[field] or '\n' in config[field]): raise ValueError('Invalid SMTP credentials')
        return config, tls_context(config.get('ca_file'))
    url = config.get('url')
    if not isinstance(url, str) or any(ord(c) < 32 for c in url):
        raise ValueError('Invalid webhook URL')
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme != 'https' or not parsed.hostname or parsed.username or parsed.password or parsed.fragment:
        raise ValueError('Webhook requires HTTPS without embedded user credentials')
    token = config.get('bearer_token')
    if token is not None and (not isinstance(token, str) or not token or any(ord(c) < 33 or ord(c) > 126 for c in token)):
        raise ValueError('Invalid bearer token')
    return config, tls_context(config.get('ca_file'))


def tls_context(ca):
    if ca is not None:
        path = Path(ca)
        if not path.is_absolute(): raise ValueError('CA path must be absolute')
        trusted_parents(path)
        regular(path)
    return ssl.create_default_context(cafile=ca)


def send_mail(config, context, body):
    message = EmailMessage()
    message['From'] = config['from']; message['To'] = ', '.join(config['to'])
    message['Message-ID'] = '<'+body['event_id']+'@monitor.local>'
    message['Subject'] = 'Monitor: '+('prueba de alertas (sin incidente)' if body.get('event')=='monitor.delivery.test' else
                                     'recuperado' if body.get('findings') == [] else 'alerta operativa')
    message.set_content('Evento de Monitor. Revisar con el responsable de operacion.\n\n'+json.dumps(body,ensure_ascii=False,indent=2))
    for attempt in range(3):
        try:
            constructor = smtplib.SMTP_SSL if config['smtp_mode']=='tls' else smtplib.SMTP
            options = {'context':context} if config['smtp_mode']=='tls' else {}
            with constructor(config['smtp_host'],config['smtp_port'],timeout=10,**options) as client:
                client.ehlo()
                if config['smtp_mode']=='starttls': client.starttls(context=context); client.ehlo()
                if config.get('smtp_username'): client.login(config['smtp_username'],config['smtp_password'])
                return not client.send_message(message)
        except smtplib.SMTPAuthenticationError:
            print('Monitor SMTP: authentication rejected.', file=sys.stderr)
            return False
        except smtplib.SMTPRecipientsRefused:
            print('Monitor SMTP: recipients rejected.', file=sys.stderr)
            return False
        except ssl.SSLCertVerificationError:
            print('Monitor SMTP: certificate verification failed.', file=sys.stderr)
            return False
        except smtplib.SMTPResponseException as error:
            if not 400 <= error.smtp_code < 500:
                print('Monitor SMTP: permanent server rejection.', file=sys.stderr)
                return False
        except (smtplib.SMTPException,OSError) as error:
            if attempt == 2:
                category = ('tls' if isinstance(error, ssl.SSLError) else
                            'timeout' if isinstance(error, TimeoutError) else
                            'smtp_protocol' if isinstance(error, smtplib.SMTPException) else 'network')
                print('Monitor SMTP: '+category+' failure.', file=sys.stderr)
        if attempt < 2: time.sleep(2 ** attempt)
    print('Monitor SMTP: delivery unavailable after bounded retries.', file=sys.stderr)
    return False


def deliver(config, context, unit, findings=None):
    event_id = str(uuid.uuid4())
    body = {'schema_version': 1, 'event_id': event_id,
                          'event': 'monitor.health.changed' if findings is not None else 'monitor.unit.failed', 'unit': unit,
                          'host': socket.gethostname(),
                          'occurred_at': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    if findings is not None: body['findings'] = findings
    if config.get('transport') == 'smtp': return send_mail(config,context,body)
    payload = json.dumps(body).encode()
    headers = {'Content-Type': 'application/json', 'Idempotency-Key': event_id}
    if config.get('bearer_token'): headers['Authorization'] = 'Bearer ' + config['bearer_token']
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect(),
                                         urllib.request.HTTPSHandler(context=context))
    for attempt in range(3):
        try:
            request = urllib.request.Request(config['url'], data=payload, headers=headers, method='POST')
            with opener.open(request, timeout=10) as response:
                if 200 <= response.status < 300:
                    return True
                return False
        except urllib.error.HTTPError as error:
            retry = error.code == 429 or 500 <= error.code < 600
            error.close()
            if not retry: return False
        except (urllib.error.URLError, TimeoutError, OSError):
            pass
        if attempt < 2: time.sleep(2 ** attempt)
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, default=Path('/etc/monitor-alert/webhook.json'))
    parser.add_argument('--unit', required=True)
    args = parser.parse_args()
    if os.geteuid() != 0: parser.error('Run as root')
    if not re.fullmatch(r'[A-Za-z0-9_.@:-]{1,180}', args.unit): parser.error('Invalid unit name')
    try:
        config, context = configuration(args.config)
        if not deliver(config, context, args.unit):
            print('Monitor alert delivery failed; check receiver and protected configuration.', file=sys.stderr)
            return 1
    except Exception:
        # Exceptions may contain a token-bearing URL. Never print their contents.
        print('Monitor alert configuration or delivery failed; no credentials logged.', file=sys.stderr)
        return 1
    print('Monitor failure event accepted by receiver; operator acknowledgement is not implied.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
