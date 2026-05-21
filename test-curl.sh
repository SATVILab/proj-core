URL=$(curl -s https://api.github.com/repos/Kornelski/cargo-deb/releases/latest | grep browser_download_url | grep -E 'x86_64-unknown-linux-musl.tar.gz' | cut -d '"' -f 4)
if [ -z "$URL" ]; then
    echo "No matching tar.gz found!"
fi
