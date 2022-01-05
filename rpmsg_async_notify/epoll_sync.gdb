# Connect to the remote target
target extended-remote 192.168.1.177:2345

# move the file to remote board
remote put ./target/aarch64-unknown-linux-gnu/debug/examples/epoll_sync /home/root/epoll_sync

# set the executable file
set remote exec-file epoll_sync

set args 10
