"""Extract unchanged C++ mapping functions into a standalone Windows reference.

python rust/tests/reference_mapping.py /tmp/padmux-mapping-reference.cpp
Compile that file with MSVC and /I <repository>/src, then run with
rust/tests/mapping-input.txt. stdout is mapping-expected.txt.
No controller writes or keyboard/mouse input: SendKey/SendInput record attempts.
"""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]
source = (root / 'src/app/main.cpp').read_text()
def part(start, end):
    return source[source.index(start):source.index(end)]

header = r'''
#include <Windows.h>
#include <array>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <map>
#include <mutex>
#include <string>
#include <thread>
#include <vector>
#include <algorithm>
#include <fstream>
#include <iostream>
#include <sstream>
#include "steam/SteamController.h"
uint64_t tickMs = 0;
struct Clock {
    using time_point = std::chrono::steady_clock::time_point;
    static time_point now() { return time_point(std::chrono::milliseconds(tickMs)); }
};
constexpr uint32_t MOUSE_LEFT = 0x20001, INHERIT = 0xffffffff;
uint32_t failKey = 0;
bool failDown = false;
int failCount = 0;
bool SendKey(uint32_t key, bool down) {
    bool ok = true;
    if (key == failKey && down == failDown && failCount > 0) { --failCount; ok = false; }
    std::cout << "K " << tickMs << " " << key << " " << down << " " << ok << "\n";
    return ok;
}
void SetStatus(const wchar_t* text) {
    std::cout << "S " << tickMs << " " << (std::wstring(text).find(L"失敗") == std::wstring::npos) << "\n";
}
UINT RecordInput(UINT count, INPUT* input, int) {
    for (UINT i=0; i<count; ++i) {
        auto& m = input[i].mi;
        if (m.dwFlags == MOUSEEVENTF_MOVE)
            std::cout << "M " << tickMs << " " << m.dx << " " << m.dy << "\n";
        else std::cout << "W " << tickMs << " " << static_cast<int32_t>(m.mouseData)
                       << " " << (m.dwFlags == MOUSEEVENTF_HWHEEL) << "\n";
    }
    return count;
}
#define SendInput RecordInput
int16_t ReadInt16(const uint8_t* p) { int16_t value; std::memcpy(&value,p,2); return value; }
uint16_t ReadUInt16(const uint8_t* p) { uint16_t value; std::memcpy(&value,p,2); return value; }
'''
models = part('enum Button : size_t {', 'struct ButtonDef {')
models += part('struct TurboSettings {', 'std::vector<Layer> liveLayers;')
globals_code = r'''
std::array<std::atomic<uint32_t>, ButtonCount> bindings{};
std::array<TurboSettings, ButtonCount> liveTurbo{};
std::array<SequenceSettings, ButtonCount> liveSequence{};
std::vector<Layer> liveLayers;
std::mutex bindingMutex;
uint64_t mappingGeneration = 0;
'''
functions = part('struct PadPressState {', 'void ControllerLoop() {')
main = r'''
int main(int argc, char** argv) {
    if (argc != 2) return 2;
    std::ifstream file(argv[1]);
    if (!file) return 3;
    HeldKeys held;
    std::array<uint32_t,ButtonCount> active{};
    std::array<bool,ButtonCount> physical{};
    std::array<TurboState,ButtonCount> turbo{};
    std::array<SequenceState,ButtonCount> sequence{};
    uint64_t applied = 0;
    PadPressState lp,rp;
    TapState lt,rt;
    StickDirections ls,rs;
    uint32_t dz[2]{12288,12288}, overlap[2]{16000,16000};
    PadMotionState motion;
    auto printPhysical = [&]() {
        uint32_t bits = 0;
        for (size_t i=0; i<ButtonCount; ++i) if (physical[i]) bits |= uint32_t(1)<<i;
        std::cout << "P " << tickMs << " " << bits << "\n";
    };
    std::string line;
    while (std::getline(file,line)) {
        if (line.empty() || line[0]=='#') continue;
        std::istringstream in(line);
        std::string cmd; in >> cmd;
        if (cmd=="reset") {
            held.clear(); active={}; physical={}; turbo={}; sequence={}; applied=0;
            lp={}; rp={}; lt={}; rt={}; ls={}; rs={}; motion={};
            dz[0]=dz[1]=12288; overlap[0]=overlap[1]=16000;
            mappingGeneration=0; liveLayers.clear(); liveTurbo={}; liveSequence={};
            for (auto& key:bindings) key=INHERIT;
            failCount=0;
        } else if (cmd=="map") {
            size_t scope,index; uint32_t key; in>>scope>>index>>key;
            if (scope==0) bindings[index]=key; else liveLayers.at(scope-1).mapping[index]=key;
        } else if (cmd=="layer") {
            Layer layer; in>>layer.trigger; liveLayers.push_back(layer);
        } else if (cmd=="turbo") {
            size_t scope,index; TurboSettings t; in>>scope>>index>>t.enabled>>t.intervalMs>>t.delayMs;
            (scope==0?liveTurbo:liveLayers.at(scope-1).turbo)[index]=t;
        } else if (cmd=="seq") {
            size_t scope,index; SequenceSettings s; in>>scope>>index>>s.enabled>>s.repeat>>s.intervalMs;
            uint32_t key; while(in>>key) s.keys.push_back(key);
            (scope==0?liveSequence:liveLayers.at(scope-1).sequence)[index]=s;
        } else if (cmd=="gen") { in>>mappingGeneration;
        } else if (cmd=="fail") { in>>failKey>>failDown>>failCount;
        } else if (cmd=="tick") {
            uint32_t bits; bool enabled; in>>tickMs>>bits>>enabled;
            for (size_t i=0;i<ButtonCount;++i) physical[i]=(bits&(uint32_t(1)<<i))!=0;
            ApplyMappings(physical,enabled,held,active,turbo,sequence,applied,
                          std::chrono::milliseconds(250),std::chrono::milliseconds(40));
        } else if (cmd=="release") { in>>tickMs; ReleaseAll(held,active);
        } else if (cmd=="stick") { size_t side; in>>side>>dz[side]>>overlap[side];
        } else if (cmd=="physical") {
            std::string hex; in>>tickMs>>hex;
            std::vector<uint8_t> report;
            for(size_t i=0;i<hex.size();i+=2)
                report.push_back(static_cast<uint8_t>(std::stoul(hex.substr(i,2),nullptr,16)));
            UpdatePhysical(report.data(),report.size(),physical,lp,rp,lt,rt,ls,rs,
                           dz[0],dz[1],overlap[0],overlap[1],Clock::now());
            printPhysical();
        } else if (cmd=="pulse") {
            in>>tickMs;
            physical[LeftPadTap]=lt.Pulsing(Clock::now()); physical[RightPadTap]=rt.Pulsing(Clock::now());
            printPhysical();
        } else if (cmd=="motion") {
            uint32_t mode,sensitivity; bool contact; int16_t x,y;
            in>>tickMs>>mode>>sensitivity>>contact>>x>>y;
            bool h = motion.Update(mode,sensitivity,contact,x,y);
            std::cout << "H " << tickMs << " " << h << "\n";
        } else { return 4; }
        if (in.bad()) return 5;
    }
}
'''
Path(sys.argv[1]).write_text(header + models + globals_code + functions + main)
