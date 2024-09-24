set -eux

(cd macros && cargo clean)
(cd native && cargo clean)
(cd neoforge-1.21 && rm -rf run bin build src/generated)
