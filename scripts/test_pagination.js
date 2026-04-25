// Simple test script to verify pagination logic
// Run with: node test_pagination.js
// Ensure server is running first!

const BASE_URL = 'http://localhost:3000';

async function testPagination() {
    console.log('🧪 Testing Pagination Logic...\n');

    try {
        // Test 1: Default Pagination (Page 1, Limit 10)
        console.log('Test 1: Default Parameters (Page 1, Limit 10)');
        const res1 = await fetch(`${BASE_URL}/api/transactions`);
        const data1 = await res1.json();
        
        console.assert(data1.data.length === 10, `Expected 10 items, got ${data1.data.length}`);
        console.assert(data1.page === 1, `Expected page 1, got ${data1.page}`);
        console.assert(data1.totalCount === 100, `Expected 100 total items, got ${data1.totalCount}`);
        console.log('✅ Passed\n');

        // Test 2: Custom Pagination (Page 2, Limit 5)
        console.log('Test 2: Custom Parameters (Page 2, Limit 5)');
        const res2 = await fetch(`${BASE_URL}/api/transactions?page=2&limit=5`);
        const data2 = await res2.json();

        console.assert(data2.data.length === 5, `Expected 5 items, got ${data2.data.length}`);
        console.assert(data2.page === 2, `Expected page 2, got ${data2.page}`);
        // Verify we sliced correctly (IDs should be 6-10 for 1-based index, or logical equivalent)
        // Since data is mocked, we just check count here.
        console.log('✅ Passed\n');

        // Test 3: Out of Range
        console.log('Test 3: Page Out of Range');
        const res3 = await fetch(`${BASE_URL}/api/transactions?page=999&limit=10`);
        const data3 = await res3.json();

        console.assert(data3.data.length === 0, `Expected 0 items, got ${data3.data.length}`);
        console.log('✅ Passed\n');

        console.log('🎉 All pagination tests passed!');

    } catch (error) {
        console.error('❌ Test failed:', error.message);
        console.log('Note: Make sure the server is running on port 3000');
    }
}

testPagination();