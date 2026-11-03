# 为 Rockchip USB 设备（VID 0x2207）安装 Windows 自带的 WinUSB 驱动，供 rkdeveloptool/libusb 使用。
#
# 1. 生成一个覆盖 rockchip-pids.txt 全部 PID 的驱动并签名、预装到驱动库（只需一次，约半分钟），
#    以后插入这些芯片的 Maskrom/Loader 设备即自动使用 WinUSB；
# 2. 已连接的 Rockchip 设备：列表内的直接改用该驱动；列表外的 PID 单独生成驱动安装。
#
# 注意：会替换 Rockchip 官方驱动（DriverAssitant）。如需再使用 RKDevTool，请重新安装官方驱动。

param(
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    (New-Object Security.Principal.WindowsPrincipal $identity).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Administrator)) {
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($Quiet) { $arguments += '-Quiet' }
    $process = Start-Process -FilePath 'powershell.exe' -ArgumentList $arguments -Verb RunAs -Wait -PassThru
    exit $process.ExitCode
}

$wdi = Join-Path $PSScriptRoot 'wdi-simple.exe'
$pidList = Join-Path $PSScriptRoot 'rockchip-pids.txt'
foreach ($file in $wdi, $pidList) {
    if (-not (Test-Path $file)) { Write-Error "找不到 $file" }
}

# 与构建时写进 INF 模板的列表相同；第一行是生成驱动时的主 PID。
$KnownPids = @(Get-Content $pidList | ForEach-Object { $_.Trim().ToUpper() } |
    Where-Object { $_ -match '^[0-9A-F]{4}$' })

$present = @()
try {
    $present = @(Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
        Where-Object { $_.InstanceId -match '^USB\\VID_2207&PID_([0-9A-F]{4})' } |
        ForEach-Object { ([regex]::Match($_.InstanceId, 'PID_([0-9A-F]{4})')).Groups[1].Value.ToUpper() } |
        Select-Object -Unique)
} catch {
    $present = @()
}

$workDirectory = Join-Path $env:TEMP 'rkflash-driver'
$failed = @()

function Install-WinUsb([string]$productId, [string]$infName) {
    & $wdi --vid 0x2207 --pid "0x$productId" --type 0 `
        --name 'Rockchip RockUSB' --manufacturer 'Rockchip' `
        --inf $infName --dest $workDirectory `
        --timeout 120000 --log 3 | Out-Null
    return $LASTEXITCODE
}

# 1. 一次生成并预装覆盖全部已知 PID 的驱动；主 PID 的设备若已连接，同时完成安装。
$primary = $KnownPids[0]
if (-not $Quiet) { Write-Host "安装 WinUSB 驱动（覆盖 $($KnownPids.Count) 个 Rockchip PID）..." }
$code = Install-WinUsb $primary 'rockchip_rockusb.inf'
if ($code -ne 0) { $failed += "驱动包（错误码 $code）" }

# 2. 已连接的设备：列表内的强制改用刚预装的驱动，列表外的单独生成驱动。
$sharedInf = Join-Path $workDirectory 'rockchip_rockusb.inf'
$listed = @($present | Where-Object { $_ -ne $primary -and $KnownPids -contains $_ })
if ($listed.Count -gt 0 -and $code -eq 0) {
    Add-Type -Namespace RkFlash -Name NewDev -MemberDefinition @'
[DllImport("newdev.dll", CharSet = CharSet.Unicode, SetLastError = true)]
public static extern bool UpdateDriverForPlugAndPlayDevices(
    IntPtr hwndParent, string hardwareId, string fullInfPath, uint installFlags, out bool rebootRequired);
'@
    foreach ($productId in $listed) {
        if (-not $Quiet) { Write-Host "已连接设备 VID_2207&PID_$productId 改用 WinUSB..." }
        $reboot = $false
        # INSTALLFLAG_FORCE = 1：即使已装有 Rockchip 官方驱动也替换为 WinUSB。
        if (-not [RkFlash.NewDev]::UpdateDriverForPlugAndPlayDevices(
                [IntPtr]::Zero, "USB\VID_2207&PID_$productId", $sharedInf, 1, [ref]$reboot)) {
            $failed += "$productId（错误码 $([Runtime.InteropServices.Marshal]::GetLastWin32Error())）"
        }
    }
}
foreach ($productId in @($present | Where-Object { $KnownPids -notcontains $_ })) {
    if (-not $Quiet) { Write-Host "已连接设备 VID_2207&PID_$productId 不在预装列表，单独安装..." }
    $code = Install-WinUsb $productId "rockchip_$($productId.ToLower()).inf"
    if ($code -ne 0) { $failed += "$productId（错误码 $code）" }
}

Remove-Item -Recurse -Force $workDirectory -ErrorAction SilentlyContinue

if ($failed.Count -gt 0) {
    Write-Warning ("以下驱动安装失败：" + ($failed -join '，'))
    if (-not $Quiet) { Read-Host '按回车键退出' | Out-Null }
    exit 1
}
if (-not $Quiet) {
    Write-Host "WinUSB 驱动安装完成，覆盖 $($KnownPids.Count) 个 Rockchip PID。请重新插拔开发板。"
    Read-Host '按回车键退出' | Out-Null
}
exit 0
