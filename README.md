# ore-miner

Build for linux:
docker build -t ore --platform linux/arm64 . 
docker cp `docker run --rm -d ore`:/target/release/ore ./bin

Check containers:
docker ps