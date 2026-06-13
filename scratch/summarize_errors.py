import json
import os

with open('server/errors.json', 'r', encoding='utf-16') as f:
    for line in f:
        try:
            msg = json.loads(line)
            if msg.get('reason') == 'compiler-message':
                message = msg['message']
                if message['level'] == 'error':
                    print(message['message'])
                    if 'spans' in message and len(message['spans']) > 0:
                        span = message['spans'][0]
                        print(f"  --> {span['file_name']}:{span['line_start']}")
        except:
            pass
