#ifndef ngspice_SIM_H
#define ngspice_SIM_H

// This file is copied directly from the ngSPICE codebase. It has not changed since version ngspice-27,
// but it could change in the future.

// Copyright 1985 - 2018, Regents of the University of California and others
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice,
// this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
// this list of conditions and the following disclaimer in the documentation
// and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
// contributors may be used to endorse or promote products derived from this
// software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
//

enum simulation_types {
  SV_NOTYPE,
  SV_TIME,
  SV_FREQUENCY,
  SV_VOLTAGE,
  SV_CURRENT,
  SV_VOLTAGE_DENSITY,
  SV_CURRENT_DENSITY,
  SV_SQR_VOLTAGE_DENSITY,
  SV_SQR_CURRENT_DENSITY,
  SV_SQR_VOLTAGE,
  SV_SQR_CURRENT,
  SV_POLE,
  SV_ZERO,
  SV_SPARAM,
  SV_TEMP,
  SV_RES,
  SV_IMPEDANCE,
  SV_ADMITTANCE,
  SV_POWER,
  SV_PHASE,
  SV_DB,
  SV_CAPACITANCE,
  SV_CHARGE
};

#endif
