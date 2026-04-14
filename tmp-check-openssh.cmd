@echo off
powershell -NoProfile -Command "Get-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0 | Format-List Name,State"
dir C:\Windows\System32\OpenSSH
