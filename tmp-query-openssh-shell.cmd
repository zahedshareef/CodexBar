@echo off
reg query HKLM\SOFTWARE\OpenSSH /v DefaultShell
reg query HKLM\SOFTWARE\OpenSSH /v DefaultShellCommandOption
