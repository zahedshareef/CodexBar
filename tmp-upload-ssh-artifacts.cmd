@echo off
set LOG=C:\Users\Administrator\sshd-debug.log
set EVT=C:\Users\Administrator\ssh-events.txt

if exist "%LOG%" (
  curl.exe -s -X POST -H "X-Filename: sshd-debug.log" --data-binary "@%LOG%" http://192.168.122.1:8000/
)

powershell -NoProfile -Command "Get-WinEvent -LogName Application -MaxEvents 50 | Where-Object {$_.ProviderName -like '*ssh*' -or $_.Message -like '*ssh*'} | Select-Object -First 20 TimeCreated,ProviderName,Id,LevelDisplayName,Message | Format-List | Out-File -Encoding utf8 'C:\Users\Administrator\ssh-events.txt'"

if exist "%EVT%" (
  curl.exe -s -X POST -H "X-Filename: ssh-events.txt" --data-binary "@%EVT%" http://192.168.122.1:8000/
)
