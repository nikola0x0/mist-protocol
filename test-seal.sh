#!/bin/bash

# SEAL Test Harness Quick Start Script

echo "🧪 Mist Protocol - SEAL Test Harness"
echo "===================================="
echo ""

# Check if we're in the right directory
if [ ! -d "backend-seal" ] || [ ! -d "frontend" ]; then
    echo "❌ Error: Must run from project root directory"
    echo "   Expected directories: backend-seal/, frontend/"
    exit 1
fi

# Function to check if a port is in use
check_port() {
    lsof -i :$1 >/dev/null 2>&1
}

echo "📋 Pre-flight checks..."
echo ""

# Check frontend port
if check_port 3000; then
    echo "⚠️  Port 3000 is in use (frontend)"
    echo "   Kill existing process? (y/n)"
    read answer
    if [ "$answer" = "y" ]; then
        lsof -ti :3000 | xargs kill -9
        echo "   ✅ Killed process on port 3000"
    fi
fi

# Check backend port
if check_port 3001; then
    echo "⚠️  Port 3001 is in use (backend)"
    echo "   Kill existing process? (y/n)"
    read answer
    if [ "$answer" = "y" ]; then
        lsof -ti :3001 | xargs kill -9
        echo "   ✅ Killed process on port 3001"
    fi
fi

echo ""
echo "🚀 Starting services..."
echo ""

# Start frontend in background
echo "1️⃣  Starting frontend (http://localhost:3000)..."
cd frontend
npm run dev > ../frontend.log 2>&1 &
FRONTEND_PID=$!
cd ..

# Wait for frontend to start
echo "   Waiting for frontend..."
sleep 5

# Check if frontend is running
if ! curl -s http://localhost:3000/ > /dev/null; then
    echo "   ❌ Frontend failed to start. Check frontend.log"
    kill $FRONTEND_PID 2>/dev/null
    exit 1
fi
echo "   ✅ Frontend running (PID: $FRONTEND_PID)"

# Start backend-seal in background
echo ""
echo "2️⃣  Starting backend-seal (http://localhost:3001)..."
cd backend-seal
cargo build --features mist-protocol >/dev/null 2>&1
cargo run --features mist-protocol --bin nautilus-server > ../backend.log 2>&1 &
BACKEND_PID=$!
cd ..

# Wait for backend to start
echo "   Waiting for backend..."
sleep 3

# Check if backend is running
if ! curl -s http://localhost:3001/ > /dev/null; then
    echo "   ❌ Backend failed to start. Check backend.log"
    kill $BACKEND_PID 2>/dev/null
    kill $FRONTEND_PID 2>/dev/null
    exit 1
fi
echo "   ✅ Backend running (PID: $BACKEND_PID)"

echo ""
echo "✅ All services running!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📖 Open the test page:"
echo "   http://localhost:3000/seal-test"
echo ""
echo "🔍 Monitor logs:"
echo "   tail -f backend.log   (backend-seal on port 3001)"
echo "   tail -f frontend.log  (frontend on port 3000)"
echo ""
echo "🛑 Stop services:"
echo "   kill $FRONTEND_PID $BACKEND_PID"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Save PIDs to file for easy cleanup
echo "$BACKEND_PID $FRONTEND_PID" > .seal-test.pids

echo "Press Ctrl+C to stop all services..."
echo ""

# Wait for user interrupt
trap "echo ''; echo '🛑 Stopping services...'; kill $BACKEND_PID $FRONTEND_PID 2>/dev/null; rm -f .seal-test.pids; echo '✅ All services stopped'; exit 0" INT

# Keep script running
wait
