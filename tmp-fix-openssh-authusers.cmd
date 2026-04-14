@echo off
icacls C:\Windows\System32\OpenSSH /grant "Authenticated Users":(OI)(CI)(RX) /T
sc.exe stop sshd
sc.exe start sshd
sc.exe query sshd
