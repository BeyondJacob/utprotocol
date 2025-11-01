#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting UT Protocol...${NC}"

# Load environment variables
set -a
source .env
set +a

# Function to kill background processes on exit
cleanup() {
    echo -e "\n${BLUE}Shutting down services...${NC}"
    kill $BACKEND_PID $FRONTEND_PID 2>/dev/null
    wait $BACKEND_PID $FRONTEND_PID 2>/dev/null
    echo -e "${GREEN}Services stopped${NC}"
    exit 0
}

trap cleanup INT TERM

# Kill any existing processes on ports 3000 and 3001
echo -e "${BLUE}Checking for existing processes...${NC}"
lsof -ti:3001 | xargs kill -9 2>/dev/null && echo -e "${GREEN}Killed process on port 3001${NC}" || true
lsof -ti:3000 | xargs kill -9 2>/dev/null && echo -e "${GREEN}Killed process on port 3000${NC}" || true
sleep 1

# Start backend
echo -e "${GREEN}Starting backend on http://127.0.0.1:3001${NC}"
cd llm-backend
cargo run &
BACKEND_PID=$!
cd ..

# Wait a bit for backend to start
sleep 2

# Check and install frontend dependencies if needed
echo -e "${BLUE}Checking frontend dependencies...${NC}"
if [ ! -d "llm-frontend/node_modules" ]; then
    echo -e "${BLUE}Installing frontend dependencies (this may take a moment)...${NC}"
    cd llm-frontend
    npm install
    cd ..
    echo -e "${GREEN}Frontend dependencies installed${NC}"
else
    echo -e "${GREEN}Frontend dependencies already installed${NC}"
fi

# Start frontend
echo -e "${GREEN}Starting frontend on http://localhost:3000${NC}"
cd llm-frontend
npm run dev &
FRONTEND_PID=$!
cd ..

echo -e "\n${GREEN}✅ Both services started!${NC}"
echo -e "Frontend: ${BLUE}http://localhost:3000${NC}"
echo -e "Backend:  ${BLUE}http://127.0.0.1:3001${NC}"
echo -e "\nPress Ctrl+C to stop both services"

# Wait for both processes
wait $BACKEND_PID $FRONTEND_PID
