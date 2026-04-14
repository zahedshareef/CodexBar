@echo off
powershell -NoProfile -Command "Get-WinEvent -LogName Application -MaxEvents 50 | Where-Object {$_.ProviderName -like '*ssh*' -or $_.Message -like '*ssh*'} | Select-Object -First 10 TimeCreated,ProviderName,Id,LevelDisplayName,Message | Format-List"
