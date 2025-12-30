%global _cross_first_party 1
Name: os
Version: 0.0
Release: 0
Summary: Bottlerocket OS packages
License: Apache-2.0 OR MIT

%description
%{summary}.

# Install section for binaries
%install
for b in \
  token-printer \
; do
install -p -m 0755 ${b} %{buildroot}%{_cross_bindir}
done

%package -n token-printer
Summary: Token printer utility

%description -n token-printer
Reads and prints the token value from configuration.

%files -n token-printer
%{_cross_bindir}/token-printer
