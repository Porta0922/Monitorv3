import glob
import os

def brute_force_imports():
    imports = """
use std::collections::HashMap;
use chrono::*;
use uuid::Uuid;
use crate::api::*;
use crate::domains::shared::*;
use crate::domains::device::models::*;
use crate::domains::activity::models::*;
use crate::domains::inventory::models::*;
use crate::domains::usb::models::*;
use crate::domains::wifi::models::*;
use crate::domains::security::models::*;
use serde_json::json;
"""
    # find all .rs files in domains
    for filepath in glob.glob('server/src/domains/**/*.rs', recursive=True):
        if 'shared.rs' in filepath or 'mod.rs' in filepath:
            continue
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # prepend imports
        # put it after the first line (usually use axum... or similar) or just at the top
        # if the file has #![...] we must put it after, but there shouldn't be any
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(imports + "\n" + content)

if __name__ == "__main__":
    brute_force_imports()
