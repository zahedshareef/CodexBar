@echo off
taskkill /IM sshd.exe /F >nul 2>&1
del C:\Users\Administrator\sshd-2222.log >nul 2>&1
start "" cmd /c "C:\Windows\System32\OpenSSH\sshd.exe -ddd -e -p 2222 > C:\Users\Administrator\sshd-2222.log 2>&1"
