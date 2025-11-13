"use client";

export function TransactionHistory() {
  const transactions = [
    {
      id: "1",
      type: "wrap",
      from: "100 SUI",
      to: "100 eSUI",
      status: "completed",
      timestamp: "2 min ago",
    },
    {
      id: "2",
      type: "swap",
      from: "50 eSUI",
      to: "125 eUSDC",
      status: "pending",
      timestamp: "5 min ago",
    },
    {
      id: "3",
      type: "unwrap",
      from: "25 eUSDC",
      to: "25 USDC",
      status: "completed",
      timestamp: "1 hour ago",
    },
  ];

  const getStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "text-green-500";
      case "pending":
        return "text-yellow-500";
      case "failed":
        return "text-red-500";
      default:
        return "text-gray-500";
    }
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case "wrap":
        return "📦";
      case "swap":
        return "🔄";
      case "unwrap":
        return "📤";
      default:
        return "•";
    }
  };

  return (
    <div className="card p-6">
      <h3 className="text-lg font-bold mb-4">Recent Activity</h3>

      <div className="space-y-2">
        {transactions.map((tx) => (
          <div
            key={tx.id}
            className="p-3 bg-[#0a0a0a] border border-[#262626] rounded-lg hover:border-[#333] transition"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-lg">{getTypeIcon(tx.type)}</span>
                <span className="font-medium capitalize">{tx.type}</span>
              </div>
              <span className={`text-xs font-medium ${getStatusColor(tx.status)}`}>
                {tx.status}
              </span>
            </div>
            <div className="flex items-center gap-2 text-xs text-gray-500">
              <span>{tx.from}</span>
              <span>→</span>
              <span>{tx.to}</span>
            </div>
            <div className="text-xs text-gray-600 mt-1">{tx.timestamp}</div>
          </div>
        ))}
      </div>

      {transactions.length === 0 && (
        <div className="text-center py-8 text-gray-500 text-sm">
          No transactions yet
        </div>
      )}
    </div>
  );
}
