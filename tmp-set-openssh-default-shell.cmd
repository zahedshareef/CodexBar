@echo off
reg add HKLM\SOFTWARE\OpenSSH /v DefaultShell /t REG_SZ /d C:\Windows\System32\cmd.exe /f
reg add HKLM\SOFTWARE\OpenSSH /v DefaultShellCommandOption /t REG_SZ /d /c /f
taskkill /IM sshd.exe /F >nul 2>&1
start "" C:\Windows\System32\OpenSSH\sshd.exe
