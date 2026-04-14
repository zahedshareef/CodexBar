@echo off
dir C:\Windows\System32\OpenSSH\install-sshd.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Windows\System32\OpenSSH\install-sshd.ps1
sc.exe query sshd
