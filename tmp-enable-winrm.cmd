@echo off
winrm quickconfig -q
netsh advfirewall firewall add rule name=winrm dir=in action=allow protocol=TCP localport=5985
sc.exe query WinRM
