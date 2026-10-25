# Installe whisper-server et llama-server (les programmes, sans modèle) pour Scriibe.
#
# Rien n'est téléchargé sans ta confirmation : le script affiche chaque fichier (nom, taille, source) et demande « o » pour continuer.
# Les programmes vont dans %LOCALAPPDATA%\Scriibe\engines\whisper et ...\llama (un dossier chacun : leurs DLL ggml*.dll portent le même
# nom mais ne sont pas de la même version).
#
# Usage (PowerShell) :   powershell -ExecutionPolicy Bypass -File installer-moteurs.ps1 [-Llama cpu|vulkan]
param([ValidateSet('cpu', 'vulkan')][string]$Llama = 'cpu')

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$engines = Join-Path $env:LOCALAPPDATA 'Scriibe\engines'

function Get-Release([string]$repo) {
    Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'scriibe-installer' }
}

function Confirm-Download([string]$name, [double]$bytes, [string]$source) {
    Write-Host ''
    Write-Host ("  Fichier : {0}" -f $name)
    Write-Host ("  Taille  : {0:N0} Mo" -f ($bytes / 1MB))
    Write-Host ("  Source  : {0}" -f $source)
    return (Read-Host '  Télécharger et installer ? (o/N)') -match '^[oOyY]'
}

function Install-Engine([string]$repo, [string]$pattern, [string]$folder, [string]$program) {
    $release = Get-Release $repo
    $asset = $release.assets | Where-Object { $_.name -match $pattern } | Select-Object -First 1
    if (-not $asset) {
        Write-Host "Aucun fichier ne correspond à « $pattern » dans la dernière version de $repo ($($release.tag_name)). Fichiers proposés :"
        $release.assets | ForEach-Object { Write-Host "  $($_.name)" }
        return
    }
    if (-not (Confirm-Download $asset.name $asset.size $asset.browser_download_url)) { Write-Host '  Passé.'; return }
    $zip = Join-Path $env:TEMP $asset.name
    $unpacked = Join-Path $env:TEMP ("scriibe-" + [guid]::NewGuid())
    Invoke-WebRequest $asset.browser_download_url -OutFile $zip
    Expand-Archive $zip -DestinationPath $unpacked
    $target = Join-Path $engines $folder
    New-Item -ItemType Directory -Force $target | Out-Null
    # Tout le contenu du zip, à plat : le programme et ses DLL doivent être côte à côte.
    Get-ChildItem $unpacked -Recurse -File | ForEach-Object { Copy-Item $_.FullName $target -Force }
    Remove-Item $zip, $unpacked -Recurse -Force
    if (Test-Path (Join-Path $target $program)) {
        Write-Host "  Installé : $(Join-Path $target $program) (version $($release.tag_name))"
    } else {
        Write-Host "  ATTENTION : $program n'est pas dans ce fichier. Contenu de $target :"
        Get-ChildItem $target | ForEach-Object { Write-Host "    $($_.Name)" }
    }
}

Write-Host 'Scriibe — installation des moteurs (programmes seulement, modèles à part).'
Install-Engine 'ggml-org/whisper.cpp' '^whisper-bin-x64\.zip$' 'whisper' 'whisper-server.exe'
Install-Engine 'ggml-org/llama.cpp' ("^llama-.*-bin-win-$Llama-x64\.zip$") 'llama' 'llama-server.exe'
Write-Host ''
Write-Host 'Terminé. Vérifie avec : scriibe verifier'
