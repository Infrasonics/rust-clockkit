#pragma once
#include "PhaseLockedClock.h"

struct ConfigReader;

std::shared_ptr<dex::PhaseLockedClock> buildPLC(ConfigReader config);

int64_t getValue(std::shared_ptr<dex::PhaseLockedClock> clock);

void setPhasePanic(std::shared_ptr<dex::PhaseLockedClock> clock, int64_t micros);

void setUpdatePanic(std::shared_ptr<dex::PhaseLockedClock> clock, int64_t micros);
