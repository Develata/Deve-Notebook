$workspace = "e:\gitclone\Deve-Note"
Get-ChildItem -Path $workspace -Recurse -Filter "*.rs" | ForEach-Object {
    $file = $_.FullName
    $dir = $_.DirectoryName
    # Calculate relative path
    $relativePath = $dir.Substring($workspace.Length).TrimStart("\")
    
    # Explicitly read with UTF8 to avoid Mojibake
    $content = Get-Content -Path $file -Raw -Encoding UTF8
    $newComment = "// $relativePath"
    
    $lines = $content -split "`r`n"
    $firstLine = $lines[0]
    
    # 定义判断逻辑：
    # 1. 是绝对路径 (e:\...)
    # 2. 是完全匹配的新相对路径
    # 3. 是旧的路径格式 (以 // 开头，且包含反斜杠 \，通常意味着它是一个路径注释)
    $isPathComment = $firstLine -match "^//\s*[a-zA-Z]:\\" -or ($firstLine.StartsWith("//") -and $firstLine.Contains("\"))

    if ($isPathComment) {
        # 如果第一行已经是路径注释，但内容不一样，则替换
        if ($firstLine -ne $newComment) {
            $lines[0] = $newComment
            $newContent = $lines -join "`r`n"
            Set-Content -Path $file -Value $newContent -NoNewline -Encoding UTF8
            Write-Host "Updated (replaced): $file" -ForegroundColor Yellow
        }
        else {
            # 如果内容一样，跳过，不输出日志或输出一条Debug日志
            # Write-Host "Skipped (up-to-date): $file" -ForegroundColor DarkGray
        }
    }
    else {
        # 第一行不是路径注释（例如是代码或普通文档注释），则插入
        $newContent = "$newComment`r`n$content"
        Set-Content -Path $file -Value $newContent -NoNewline -Encoding UTF8
        Write-Host "Updated (prepended): $file" -ForegroundColor Green
    }
