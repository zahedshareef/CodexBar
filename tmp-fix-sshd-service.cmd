@echo off
C:\Windows\System32\OpenSSH\ssh-keygen.exe -A
sc.exe config sshd binPath= "C:\Windows\System32\OpenSSH\sshd.exe"
sc.exe start sshd
sc.exe query sshd
