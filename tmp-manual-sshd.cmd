@echo off
C:\Windows\System32\OpenSSH\ssh-keygen.exe -A
sc.exe create sshd binPath= "C:\Windows\System32\OpenSSH\sshd.exe" start= auto
sc.exe start sshd
netsh advfirewall firewall add rule name=sshd dir=in action=allow protocol=TCP localport=22
sc.exe query sshd
