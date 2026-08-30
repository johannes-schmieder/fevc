function value = corr(left,right)
% Compatibility implementation for the maintained code's printed diagnostic.
if nargin==1
    value = corrcoef(left);
else
    combined = corrcoef(left,right);
    value = combined(1,2);
end
end
