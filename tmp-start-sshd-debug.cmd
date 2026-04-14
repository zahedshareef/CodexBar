@echo off
taskkill /IM sshd.exe /F
cmd /c start "" /b C:\Windows\System32\OpenSSH\sshd.exe -ddd -e > C:\Users\Administrator\sshd-debug.log 2>&1
