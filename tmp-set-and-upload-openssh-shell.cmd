@echo off
reg add HKLM\SOFTWARE\OpenSSH /v DefaultShell /t REG_SZ /d C:\Windows\System32\cmd.exe /f
reg add HKLM\SOFTWARE\OpenSSH /v DefaultShellCommandOption /t REG_SZ /d /c /f
taskkill /IM sshd.exe /F >nul 2>&1
start "" C:\Windows\System32\OpenSSH\sshd.exe
(
  reg query HKLM\SOFTWARE\OpenSSH /v DefaultShell
  reg query HKLM\SOFTWARE\OpenSSH /v DefaultShellCommandOption
) > C:\Users\Administrator\openssh-shell.txt 2>&1
curl.exe -s -X POST -H "X-Filename: openssh-shell.txt" --data-binary "@C:\Users\Administrator\openssh-shell.txt" http://192.168.122.1:8000/
