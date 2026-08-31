function value = corr(left,right)
% Compatibility for the maintained routine's printed AKM diagnostic only.
left = double(left(:));
right = double(right(:));
assert(numel(left)==numel(right) && numel(left)>=2);
left = left-mean(left);
right = right-mean(right);
value = (left'*right)/sqrt((left'*left)*(right'*right));
end
