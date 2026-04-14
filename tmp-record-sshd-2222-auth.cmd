@echo off
del C:\Users\Administrator\sshd-2222.log >nul 2>&1
C:\Windows\System32\OpenSSH\sshd.exe -ddd -e -p 2222 > C:\Users\Administrator\sshd-2222.log 2>&1
