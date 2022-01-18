# How to manually configure routes with tun devices
## On macOS
### setup
```
# Config tun device
ifconfig utun001 inet 10.1.2.9 netmask 255.255.255.0 10.1.2.1
# Delete original default routes 
route delete -inet default
# Add route through tun
route add -inet default 10.1.2.1
route add -inet default 192.168.12.1 -ifscope en0
```
### cleanup
```
route delete -inet default 10.1.2.1
route delete -inet default 192.168.12.1 -ifscope en0
# Add back original routes
route add -inet default 192.168.12.1
```

## on Linux
### setup
```
# Config tun device
ip addr add 10.1.2.9/24 dev tun01
# Delete original default routes
ip route delete default table main
# Add route through tun
ip route add default via 10.1.2.1 table main
ip route add default via 192.168.12.1 dev eth0 table default 
```
### cleanup
```
ip route delete default table main
ip route delete default table default
# Add back original routes
ip route add default via 192.168.12.1 table main
```

## on Windows
### setup
```
# Config tun device
netsh interface ipv4 set address wintun1  static 10.1.2.9 255.255.255.0 10.1.2.1 store=active
# Delete original default routes
netsh interface ipv4 delete route 0.0.0.0/0 if=1 store=active
# Add route through tun
netsh interface ipv4 add route 0.0.0.0/0 15 10.1.2.1 store=active
netsh interface ipv4 add route 0.0.0.0/0 1 192.168.12.1 store=active
```
### cleanup
```
netsh interface ipv4 delete route 0.0.0.0/0 store=active
netsh interface ipv4 add route 0.0.0.0/0 1 192.168.12.1 store=active
```