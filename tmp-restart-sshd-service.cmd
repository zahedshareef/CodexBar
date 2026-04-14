@echo off
taskkill /IM sshd.exe /F >nul 2>&1
sc.exe stop sshd >nul 2>&1
sc.exe start sshd
sc.exe query sshd
