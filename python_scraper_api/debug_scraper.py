#!/usr/bin/env python3
"""
Test script to run the Rust Walmart scraper with debugging.
"""

import time
import sys
import os
from pathlib import Path
import requests

# Add the current directory to Python path to import rust_scrapwal
current_dir = Path(__file__).parent
sys.path.insert(0, str(current_dir))

def test_url_accessibility():
    """Test if the URL is accessible and what we get back."""
    test_url = "https://www.walmart.com/seller/102616245"
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
    }
    
    print(f"Testing URL accessibility: {test_url}")
    try:
        response = requests.get(test_url, headers=headers, timeout=30)
        print(f"Status code: {response.status_code}")
        print(f"Content length: {len(response.text)}")
        
        # Check if __NEXT_DATA__ is in the response
        if "__NEXT_DATA__" in response.text:
            print("✅ __NEXT_DATA__ found in response")
        else:
            print("❌ __NEXT_DATA__ NOT found in response")
            
        # Check for common bot detection patterns
        if "blocked" in response.text.lower() or "captcha" in response.text.lower():
            print("⚠️  Possible bot detection detected")
        
        # Save a sample of the response for inspection
        with open("debug_response.html", "w", encoding="utf-8") as f:
            f.write(response.text)
        print("Response saved to debug_response.html for inspection")
        
    except Exception as e:
        print(f"❌ Error accessing URL: {e}")

def run_single_test():
    """Run a single test to debug the issue."""
    try:
        import rust_scrapwal
        print("✅ Successfully imported rust_scrapwal module")
    except ImportError as e:
        print(f"❌ Error: Could not import rust_scrapwal module: {e}")
        return
    
    test_url = "https://www.walmart.com/seller/102616245"
    print(f"\nTesting with URL: {test_url}")
    
    try:
        print("Starting scraper...")
        start_time = time.time()
        output_file = rust_scrapwal.rs_run_scraper(test_url)
        duration = time.time() - start_time
        
        print(f"✅ Test completed successfully in {duration:.2f}s")
        print(f"Output file: {output_file}")
        
    except Exception as e:
        duration = time.time() - start_time
        print(f"❌ Test failed after {duration:.2f}s: {e}")

if __name__ == "__main__":
    print("=== Debugging Walmart Scraper ===")
    
    # First test URL accessibility
    test_url_accessibility()
    
    print("\n" + "=" * 50)
    
    # Then run the scraper
    run_single_test()
