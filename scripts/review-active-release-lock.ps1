function Enter-ReviewActiveReleaseLock {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Owner,
        [int]$TimeoutSeconds = 1800
    )

    $mutexName = "Global\FinancialCrisisReviewActiveRelease"
    $mutex = [System.Threading.Mutex]::new($false, $mutexName)
    try {
        if (-not $mutex.WaitOne([TimeSpan]::FromSeconds($TimeoutSeconds))) {
            $mutex.Dispose()
            throw "Timed out waiting for review active-release lock ($Owner). Another release review/probability-slice job may still be running."
        }
    } catch {
        $mutex.Dispose()
        throw
    }

    Write-Host "Acquired review active-release lock ($Owner)."
    return $mutex
}

function Exit-ReviewActiveReleaseLock {
    param(
        [Parameter(Mandatory = $false)]
        [System.Threading.Mutex]$Mutex,
        [Parameter(Mandatory = $true)]
        [string]$Owner
    )

    if ($null -eq $Mutex) {
        return
    }

    try {
        [void]$Mutex.ReleaseMutex()
        Write-Host "Released review active-release lock ($Owner)."
    } finally {
        $Mutex.Dispose()
    }
}
