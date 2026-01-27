fn run_test() {
    print("Testing HTTP GET...");
    let response = http_get("https://httpbin.org/get");
    print("Response received (length: " + response.len() + ")");
    
    // Simple verification
    if response.contains("httpbin.org") {
        print("HTTP GET Verification: PASS");
    } else {
        print("HTTP GET Verification: FAIL");
        return "FAIL";
    }

    return "SUCCESS";
}

// Export function for runner
run_test();
