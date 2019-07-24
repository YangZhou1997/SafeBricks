set -e

#Build custom runner
cd runner
cargo +nightly build
cd -

#Build APP
cd app
cargo +nightly build --target=x86_64-fortanix-unknown-sgx
cd -

#Convert the APP
ftxsgx-elf2sgxs app/target/x86_64-fortanix-unknown-sgx/debug/app --heap-size 0x1500000 --stack-size 0x1500000 --threads 2 --debug

#Execute
env RUST_BACKTRACE=1 runner/target/debug/runner app/target/x86_64-fortanix-unknown-sgx/debug/app.sgxs
