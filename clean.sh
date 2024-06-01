set -eux

(cd macros && cargo clean)
(cd native && cargo clean)
(cd forge-1.20.1 && rm -rf run bin build src/generated)
