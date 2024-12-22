set -eux
(cd native && cargo build --release --target x64-custom.json)
(cd native && cargo build --release --target aarch64-custom.json)
python pack-native.py
(cd forge-1.20.1 && ./gradlew assemble)
