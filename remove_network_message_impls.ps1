# Script to remove all impl NetworkMessage blocks from Rust files

$files = Get-ChildItem -Path "plugins\src" -Recurse -Filter "*.rs" | Where-Object {
    (Get-Content $_.FullName -Raw) -match "impl NetworkMessage"
}

foreach ($file in $files) {
    Write-Host "Processing: $($file.FullName)"
    
    $content = Get-Content $file.FullName -Raw
    
    # Remove impl NetworkMessage blocks with const NAME
    # Pattern matches: impl NetworkMessage for Type { const NAME: &'static str = "..."; }
    $pattern = 'impl\s+NetworkMessage\s+for\s+\w+\s*\{\s*const\s+NAME:\s*&''static\s+str\s*=\s*"[^"]+"\s*;\s*\}'
    $content = $content -replace $pattern, ''
    
    # Also handle multi-line versions
    $pattern2 = '(?s)impl\s+NetworkMessage\s+for\s+\w+\s*\{\s*const\s+NAME:\s*&''static\s+str\s*=\s*"[^"]+"\s*;\s*\}\s*'
    $content = $content -replace $pattern2, ''
    
    # Remove the NetworkMessage import from pl3xus_common if it exists
    $content = $content -replace 'use\s+pl3xus_common::\{([^}]*),?\s*NetworkMessage\s*,?\s*([^}]*)\};', 'use pl3xus_common::{$1$2};'
    $content = $content -replace 'use\s+pl3xus_common::\{NetworkMessage\s*,?\s*([^}]*)\};', 'use pl3xus_common::{$1};'
    $content = $content -replace 'use\s+pl3xus_common::\{([^}]*),?\s*NetworkMessage\};', 'use pl3xus_common::{$1};'
    $content = $content -replace 'use\s+pl3xus_common::NetworkMessage;', ''
    
    # Clean up empty use statements
    $content = $content -replace 'use\s+pl3xus_common::\{\s*\};', ''
    
    # Clean up multiple blank lines
    $content = $content -replace '(\r?\n){3,}', "`n`n"
    
    Set-Content -Path $file.FullName -Value $content -NoNewline
    Write-Host "  Updated: $($file.Name)"
}

Write-Host "`nDone! Processed $($files.Count) files."

