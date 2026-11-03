#!/usr/bin/env python3
"""Produce an Argon2id hash without placing a password in arguments/history.
Requires argon2-cffi on the administrator workstation.
"""
import getpass
from argon2 import PasswordHasher, Type
password=getpass.getpass('Password: ')
if not password or password!=getpass.getpass('Confirm password: '):
    raise SystemExit('Empty password or confirmation mismatch')
print(PasswordHasher(type=Type.ID).hash(password))
