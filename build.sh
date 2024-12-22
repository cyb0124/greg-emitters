set -eux
(cd native && cargo build --release --target x64-custom.json)
(cd native && cargo build --release --target aarch64-custom.json)
python pack-native.py
(cd neoforge-1.21 && ./gradlew assemble)
