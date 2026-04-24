
'use client';

import React, { useState } from 'react';

export default function TokenSearchModal() {
  const [searchTerm, setSearchTerm] = useState('');
  const [showImportUI, setShowImportUI] = useState(false);
  const [isCheckboxChecked, setIsCheckboxChecked] = useState(false);

  const handleSearch = (e: React.ChangeEvent<HTMLInputElement>) => {
    const term = e.target.value;
    setSearchTerm(term);

    if (term.length === 56) {
      // In a real app, you would search for the token here.
      // For this example, we'll just show the import UI.
      setShowImportUI(true);
    } else {
      setShowImportUI(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-gray-800 bg-opacity-75 flex items-center justify-center">
      <div className="bg-white rounded-lg p-6 w-full max-w-md">
        <h2 className="text-xl font-bold mb-4">Select a token</h2>
        <input
          type="text"
          placeholder="Search by name or paste address"
          className="w-full p-2 border rounded"
          value={searchTerm}
          onChange={handleSearch}
        />
        {showImportUI && (
          <div className="mt-4 p-4 border border-red-500 rounded">
            <div className="flex items-center justify-between">
              <span className="font-bold text-lg">{`${searchTerm.slice(
                0,
                6
              )}...${searchTerm.slice(-4)}`}</span>
              <span className="text-red-500 font-bold">Unknown Asset</span>
            </div>
            <div className="mt-4 flex items-center">
              <input
                type="checkbox"
                id="risk-checkbox"
                checked={isCheckboxChecked}
                onChange={() => setIsCheckboxChecked(!isCheckboxChecked)}
              />
              <label htmlFor="risk-checkbox" className="ml-2 text-sm">
                I understand the risks of trading unverified tokens.
              </label>
            </div>
            <button
              className="mt-4 w-full p-2 bg-blue-500 text-white rounded disabled:bg-gray-400"
              disabled={!isCheckboxChecked}
            >
              Import
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
