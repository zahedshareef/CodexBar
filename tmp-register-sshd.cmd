@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File %WINDIR%\System32\OpenSSH\install-sshd.ps1
sc.exe config sshd start= auto
sc.exe start sshd
netsh advfirewall firewall add rule name=sshd dir=in action=allow protocol=TCP localport=22
sc.exe query sshd
