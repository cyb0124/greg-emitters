import base64
import json
import os

OUT_PATHS = ['forge-1.20.1/src/generated/resources/native.json']
x64_map = open('native/target/x64.txt', 'r').readlines()
x64_bin = open('native/target/x64-custom/release/native', 'rb').read()
aarch64_map = open('native/target/aarch64.txt', 'r').readlines()
aarch64_bin = open('native/target/aarch64-custom/release/native', 'rb').read()

def find_symbol(linker_map, name):
  lines = list(filter(lambda x: name in x, linker_map))
  assert len(lines) == 1
  line = lines[0].strip()
  pos = line.find(' ')
  addr = int(line[0:pos], base=16)
  line = line[pos:].strip()
  line = line[line.find(' '):].strip()
  size = int(line[0:line.find(' ')], base=16)
  return addr, size

result = dict()
for map, bin, prefix in [(x64_map, x64_bin, "x64"), (aarch64_map, aarch64_bin, "aarch64")]:
  reloc, reloc_size = find_symbol(map, ' out_reloc')
  result[prefix + '-b'] = base64.b64encode(bin[:reloc]).decode()
  exec_size, _ = find_symbol(map, ' out_rw')
  assert exec_size % 4096 == 0
  result[prefix + '-x'] = exec_size // 4096
  relocs = []
  for i in range(reloc, reloc + reloc_size, 24):
    cur = bin[i:i+24]
    offset = int.from_bytes(cur[0:8], 'little')
    kind = int.from_bytes(cur[8:16], 'little')
    assert kind == 8 or kind == 1027
    addend = int.from_bytes(cur[16:], 'little')
    relocs.append((offset << 32) | addend)
  result[prefix + '-r'] = relocs

win, _ = find_symbol(x64_map, ' entry_win64')
result['x64-w'] = win
result['aarch64-w'] = 0

for path in OUT_PATHS:
  os.makedirs(os.path.dirname(path), exist_ok=True)
  with open(path, 'w') as f:
    f.write(json.dumps(result))
