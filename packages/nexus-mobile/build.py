#!/usr/bin/env python3
import os

BASE = '/home/workspace/Nexus/packages/nexus-mobile/app'
os.makedirs(f'{BASE}/lib', exist_ok=True)
os.makedirs(f'{BASE}/screens', exist_ok=True)
os.makedirs(f'{BASE}/components', exist_ok=True)

files = {}
