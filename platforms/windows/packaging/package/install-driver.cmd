@echo off
rem Install the WinUSB driver for Rockchip devices (asks for administrator rights)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0driver\install-driver.ps1"
