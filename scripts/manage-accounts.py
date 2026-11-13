#!/usr/bin/env python3
"""Offline account administration on the Linux production host (root only).
Secrets are prompted, hashed via the server's stdin, and never printed.
"""
import argparse, base64, datetime, fcntl, getpass, json, os, secrets, stat, subprocess, tempfile
from pathlib import Path
from urllib.parse import quote

def private_write(path, value, mode=0o600):
    fd, name = tempfile.mkstemp(prefix='.accounts-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as stream:
            stream.write(value); stream.flush(); os.fsync(stream.fileno())
            os.fchmod(stream.fileno(), mode)
        os.replace(name, path)
        directory = os.open(path.parent, os.O_DIRECTORY)
        try: os.fsync(directory)
        finally: os.close(directory)
    finally:
        if os.path.exists(name): os.unlink(name)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--users-file', type=Path, required=True)
    executor=parser.add_mutually_exclusive_group(required=True)
    executor.add_argument('--server-bin', type=Path)
    executor.add_argument('--docker-container')
    executor.add_argument('--docker-image', help='Hash in a disposable offline container; no shell required')
    parser.add_argument('action', choices=['create','disable','enable','role','reset-password','reset-mfa'])
    parser.add_argument('username')
    parser.add_argument('--role', choices=['viewer','operator','admin'])
    parser.add_argument('--provisioning-file', type=Path, help='New file inside a root-only directory; contains MFA secret URI')
    parser.add_argument('--emergency', action='store_true', help='Create a disabled administrator with independent MFA')
    args=parser.parse_args()
    if os.geteuid()!=0: parser.error('Run on the production Linux host as root')
    target=args.users_file.absolute()
    if target.is_symlink() or not target.parent.is_dir(): parser.error('Use a regular file in an existing protected directory')
    parent=target.parent.stat()
    if not any(p.stat().st_uid==0 and stat.S_IMODE(p.stat().st_mode)&0o077==0 for p in target.parents): parser.error('A root-only ancestor directory (0700) is required to protect account secrets')
    if parent.st_uid!=0 or stat.S_IMODE(parent.st_mode)&0o022: parser.error('Account directory must be root owned and not writable by others')
    if not args.username or args.username.strip()!=args.username or len(args.username.encode())>100 or any(ord(c)<32 or ord(c)==127 for c in args.username): parser.error('Invalid username')
    if args.emergency and args.action!='create': parser.error('--emergency only applies to create')
    lock=os.open(str(target)+'.lock', os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW, 0o600)
    with os.fdopen(lock,'w') as lock_file:
        fcntl.flock(lock_file,fcntl.LOCK_EX)
        accounts=json.loads(target.read_text()) if target.exists() else []
        account=next((a for a in accounts if a['username']==args.username),None)
        if args.action=='create':
            if account or len(accounts)>=1000: parser.error('Account already exists or account limit reached')
            account={'username':args.username,'role':'admin' if args.emergency else args.role or 'viewer','disabled':args.emergency}
            accounts.append(account)
        elif account is None: parser.error('Unknown account')
        if args.action=='role':
            if not args.role: parser.error('--role is required')
            account['role']=args.role
        if args.action in ['create','reset-password']:
            password=getpass.getpass('New password (12+ characters): ')
            if len(password)<12 or len(password.encode())>4096 or password!=getpass.getpass('Confirm password: '): parser.error('Invalid password or confirmation')
            command=[str(args.server_bin),'--hash-password-stdin'] if args.server_bin else ['docker','exec','-i',args.docker_container,'/usr/local/bin/activity-monitor-server','--hash-password-stdin']
            if args.docker_image:
                if args.docker_image.startswith('-') or any(c.isspace() for c in args.docker_image): parser.error('Invalid image reference')
                command=['docker','run','--rm','-i','--network','none','--read-only','--cap-drop','ALL',
                         '--security-opt','no-new-privileges:true','--user','65532:65532',
                         '--entrypoint','/usr/local/bin/activity-monitor-server',args.docker_image,'--hash-password-stdin']
            result=subprocess.run(command,input=password,text=True,capture_output=True,timeout=60)
            if result.returncode or not result.stdout.strip().startswith('$argon2id$'): parser.error('Password hashing failed; no account changes saved')
            account['password_hash']=result.stdout.strip()
        if args.action=='disable': account['disabled']=True
        if args.action=='enable': account['disabled']=False
        mfa=args.action=='reset-mfa' or (account['role']=='admin' and not account.get('totp_secret'))
        if mfa:
            provision=args.provisioning_file
            if provision is None: parser.error('--provisioning-file is required for new MFA')
            if provision.exists() or provision.is_symlink(): parser.error('Provisioning file must not exist')
            info=provision.parent.stat()
            if info.st_uid!=0 or stat.S_IMODE(info.st_mode)&0o077: parser.error('Provisioning directory must be root-only (0700)')
            account['totp_secret']=base64.b32encode(secrets.token_bytes(32)).decode().rstrip('=')
            uri='otpauth://totp/'+quote('Monitor:'+args.username,safe='')+'?issuer=Monitor&algorithm=SHA1&digits=6&period=30&secret='+account['totp_secret']
            fd=os.open(provision,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
            with os.fdopen(fd,'w') as output: output.write(uri+'\n'); output.flush(); os.fsync(output.fileno())
        account['session_epoch']=int(account.get('session_epoch',0))+1
        event={'timestamp':datetime.datetime.now(datetime.timezone.utc).isoformat(),'operator_uid':os.getuid(),'action':args.action,'username':args.username,'role':account['role']}
        audit_fd=os.open(str(target)+'.audit.jsonl',os.O_WRONLY|os.O_APPEND|os.O_CREAT|os.O_NOFOLLOW,0o600)
        with os.fdopen(audit_fd,'a') as audit:
            audit.write(json.dumps({**event,'result':'requested'})+'\n'); audit.flush(); os.fsync(audit.fileno())
            # Production directory is a directory bind mount. 0444 is needed by UID 65532;
            # its root-only ancestor protects the host copy.
            private_write(target,json.dumps(accounts,indent=2)+'\n',stat.S_IMODE(target.stat().st_mode) if target.exists() else 0o444)
            audit.write(json.dumps({**event,'result':'applied'})+'\n'); audit.flush(); os.fsync(audit.fileno())
    print('Account configuration updated. Verify access and store MFA provisioning securely; no secret was printed.')

if __name__=='__main__':
    try: main()
    except (OSError,ValueError,KeyError,subprocess.SubprocessError): raise SystemExit('Account update failed; inspect protected configuration and audit state before retrying.')
