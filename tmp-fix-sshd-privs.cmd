@echo off
taskkill /IM sshd.exe /F
sc.exe config sshd obj= LocalSystem
sc.exe privs sshd SeAssignPrimaryTokenPrivilege/SeTcbPrivilege/SeBackupPrivilege/SeRestorePrivilege/SeImpersonatePrivilege
sc.exe start sshd
sc.exe query sshd
