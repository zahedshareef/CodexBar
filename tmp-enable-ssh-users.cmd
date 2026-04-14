@echo off
net.exe user Administrator /active:yes
net.exe user fsos /active:yes
net.exe user Administrator codexbar234A
net.exe user fsos codexbar234A
taskkill /IM sshd.exe /F >nul 2>&1
start "" C:\Windows\System32\OpenSSH\sshd.exe
