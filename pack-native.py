import os

OUT_DIRS = ['forge-1.20.1/src/generated/resources/']
ARCHS = ['x64', 'aarch64']

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

for dir in OUT_DIRS: os.makedirs(os.path.dirname(dir), exist_ok=True)

for arch in ARCHS:
  map = open(f'native/target/{arch}.txt', 'r').readlines()
  bin = open(f'native/target/{arch}-custom/release/native', 'rb').read()
  reloc, reloc_size = find_symbol(map, ' out_reloc')
  out = bytearray(bin[:reloc])
  for i in range(reloc, reloc + reloc_size, 24):
    cur = bin[i:i+24]
    offset = int.from_bytes(cur[0:8], 'little')
    kind = int.from_bytes(cur[8:16], 'little')
    assert kind == 8 or kind == 1027
    addend = int.from_bytes(cur[16:], 'little')
    out += offset.to_bytes(4, 'big')
    out += addend.to_bytes(4, 'big')
  exec_size, _ = find_symbol(map, ' out_rw')
  assert exec_size % 4096 == 0
  out += (exec_size // 4096).to_bytes(4, 'big')
  win = find_symbol(map, ' entry_win64')[0] if arch == 'x64' else 0
  out += win.to_bytes(4, 'big')
  out += reloc.to_bytes(4, 'big')
  for dir in OUT_DIRS:
    with open(f'{dir}{arch}.bin', 'wb') as f:
      f.write(out)
