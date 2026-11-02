# 为 Rockchip USB 设备（VID 0x2207）安装 Windows 自带的 WinUSB 驱动，供 rkdeveloptool/libusb 使用。
#
# 1. 已连接的 Rockchip 设备：按实际 PID 安装并立即生效；
# 2. 常见新款芯片的 Maskrom/Loader PID：预装到驱动库，以后插入即自动使用 WinUSB。
#    每个 PID 都要单独生成并签名驱动（约半分钟），因此只预装少量常用 PID；
#    其他芯片插入开发板后运行开始菜单中的“安装或修复 USB 驱动”即可。
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

# RK3588/RK3588S、RK3566/RK3568、RK3399、RK3328 的 Maskrom/Loader PID。
$KnownPids = @('350B', '350A', '330C', '320C')

$wdi = Join-Path $PSScriptRoot 'wdi-simple.exe'
if (-not (Test-Path $wdi)) {
    Write-Error "找不到 $wdi"
}

$present = @()
try {
    $present = Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
        Where-Object { $_.InstanceId -match '^USB\\VID_2207&PID_([0-9A-F]{4})' } |
        ForEach-Object { ([regex]::Match($_.InstanceId, 'PID_([0-9A-F]{4})')).Groups[1].Value.ToUpper() }
} catch {
    $present = @()
}

$productIds = @($present) + $KnownPids | Select-Object -Unique
$workDirectory = Join-Path $env:TEMP 'rkflash-driver'
$failed = @()

foreach ($productId in $productIds) {
    $state = if ($present -contains $productId) { '已连接' } else { '预装' }
    if (-not $Quiet) { Write-Host "安装 WinUSB：VID_2207&PID_$productId（$state）..." }
    & $wdi --vid 0x2207 --pid "0x$productId" --type 0 `
        --name "Rockchip RockUSB ($productId)" --manufacturer 'Rockchip' `
        --inf "rockchip_$($productId.ToLower()).inf" --dest $workDirectory `
        --timeout 120000 --log 3 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        $failed += "$productId（错误码 $LASTEXITCODE）"
    }
}

Remove-Item -Recurse -Force $workDirectory -ErrorAction SilentlyContinue

if ($failed.Count -gt 0) {
    Write-Warning ("以下 PID 安装失败：" + ($failed -join '，'))
    if (-not $Quiet) { Read-Host '按回车键退出' | Out-Null }
    exit 1
}
if (-not $Quiet) {
    Write-Host "WinUSB 驱动安装完成，共 $($productIds.Count) 个 PID。请重新插拔开发板。"
    Read-Host '按回车键退出' | Out-Null
}
exit 0
