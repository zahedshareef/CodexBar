@echo off
if exist C:\Users\Administrator\sshd-2222.log (
  curl.exe -s -X POST -H "X-Filename: sshd-2222.log" --data-binary "@C:\Users\Administrator\sshd-2222.log" http://192.168.122.1:8000/
)
