set -eux
(cd native && cargo build --release --target x64-custom.json)
(cd native && cargo build --release --target aarch64-custom.json)
mkdir -p neoforge-1.21/src/generated/resources
cp native/target/x64-custom/release/native neoforge-1.21/src/generated/resources/x64.bin
cp native/target/aarch64-custom/release/native neoforge-1.21/src/generated/resources/aarch64.bin
(cd neoforge-1.21 && ./gradlew assemble)
